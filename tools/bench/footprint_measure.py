#!/usr/bin/env python3
"""Measure the size and the memory of proj-rs against PROJ 9.9.0 and write bench/footprint.json.

Called by bench/footprint.sh, which builds the programs and gives their paths. Linux only: sizes
come from `ldd` and `strip`, peak memory from the programs' own `VmHWM` and from valgrind's massif.
Standard library only.
"""
import argparse
import json
import os
import platform
import re
import shutil
import statistics
import subprocess
import tempfile
from datetime import datetime, timezone
from pathlib import Path

import history

# Shared objects every Linux system has before anything is installed: the C library and the
# unwinder. Everything else `ldd` lists for a program or a library is counted, on both sides.
BASE_SYSTEM = re.compile(r"^(linux-vdso|ld-linux|libc\.|libm\.|libdl\.|libpthread\.|librt\.|libutil\.|libgcc_s\.)")

SCENARIOS = [
    ("empty", "process start, nothing created"),
    ("create", "create the 4326 → 2154 transform"),
    ("points", "create + 1,000,000 points, one at a time"),
    ("array", "create + 1,000,000 points, one array"),
    ("multi", "4267 → 4326 (several candidates) + 1,000,000 points, one at a time"),
]
VARIANTS = ["proj", "proj_full", "capi", "ours"]
RUNS = 5
SUM_TOLERANCE = 1e-6


def run(cmd, **kwargs):
    return subprocess.run(cmd, check=True, text=True, capture_output=True, **kwargs).stdout


def shared_objects(binary, lib_dir=None):
    """The shared objects `ldd` resolves for `binary`: (name, real path), base system left out."""
    env = dict(os.environ)
    if lib_dir:
        env["LD_LIBRARY_PATH"] = str(lib_dir)
    found = []
    for line in run(["ldd", str(binary)], env=env).splitlines():
        match = re.match(r"\s*(\S+) => (\S+) \(", line)
        if match and not BASE_SYSTEM.match(match.group(1)):
            found.append((match.group(1), Path(match.group(2)).resolve()))
    return found


def stripped_size(path, scratch):
    out = Path(scratch) / (Path(path).name + ".stripped")
    run(["strip", "--strip-unneeded", "-o", str(out), str(path)])
    return out.stat().st_size


def file_entry(path, scratch, role):
    path = Path(path)
    return {
        "name": path.name,
        "role": role,
        "bytes": path.stat().st_size,
        "stripped_bytes": path.stat().st_size if role == "proj.db" else stripped_size(path, scratch),
    }


def contains(binary, db):
    """True when the bytes of `db` sit unchanged inside `binary` (`include_bytes!`)."""
    data, head = Path(binary).read_bytes(), Path(db).read_bytes()
    return data.find(head[:1 << 20]) >= 0 and data.find(head[-(1 << 20):]) >= 0


def measure_sizes(args, scratch):
    files = {}

    def add(key, path, role):
        files[key] = file_entry(path, scratch, role)
        return files[key]

    def dependencies(binary, lib_dir, own_lib=None):
        out = []
        for name, real in shared_objects(binary, lib_dir):
            if own_lib and name.startswith(own_lib):
                continue
            out.append(add(f"{binary.name}:{name}", real, "dependency"))
        return out

    db_bytes = args.rs_db.stat().st_size
    installs = []

    def install(key, label, side, code, db, libs, note):
        part = lambda items, field: sum(item[field] for item in items)
        parts = {
            "code": {"items": [i["name"] for i in code], "bytes": part(code, "bytes"),
                     "stripped_bytes": part(code, "stripped_bytes")},
            "db": db,
            "libs": {"items": [i["name"] for i in libs], "bytes": part(libs, "bytes"),
                     "stripped_bytes": part(libs, "stripped_bytes")},
        }
        installs.append({
            "id": key, "label": label, "side": side, "note": note, "parts": parts,
            "bytes": sum(p["bytes"] for p in parts.values()),
            "stripped_bytes": sum(p["stripped_bytes"] for p in parts.values()),
        })

    for key, label, c_prog, lib_dir, db, note in (
        ("proj", "PROJ 9.9.0, the benchmark build", args.c_proj, args.proj_lib, args.proj_db,
         "libproj built with TIFF and curl off, as in the speed comparison"),
        ("proj_full", "PROJ 9.9.0 with TIFF and curl", args.c_proj_full, args.proj_full_lib, args.proj_full_db,
         "libproj built with TIFF and curl on, as distributions build it"),
    ):
        program = add(f"{key}:program", c_prog, "C program")
        libproj = add(f"{key}:libproj", next(lib_dir.glob("libproj.so.*.*.*")), "libproj")
        data = add(f"{key}:proj.db", db, "proj.db")
        libs = dependencies(c_prog, lib_dir, own_lib="libproj")
        install(key, label, "proj", [program, libproj],
                {"items": ["proj.db"], "bytes": data["bytes"], "stripped_bytes": data["bytes"]}, libs, note)

    capi_lib = args.capi_lib
    program = add("capi:program", args.c_capi, "C program")
    libproj = add("capi:libproj", capi_lib, "libproj of proj-rs")
    if not contains(capi_lib, args.rs_db):
        raise SystemExit("footprint: the capi library does not hold proj.db unchanged")
    libs = dependencies(args.c_capi, capi_lib.parent, own_lib="libproj")
    install("capi", "proj-rs C library as libproj", "ours", [program],
            {"items": ["proj.db, inside libproj"], "bytes": db_bytes, "stripped_bytes": db_bytes}, libs,
            "capi/tests/dist.sh --release; the same C program, proj.db inside the library")
    installs[-1]["parts"]["code"]["items"].append("libproj.so (without proj.db)")
    for field in ("bytes", "stripped_bytes"):
        installs[-1]["parts"]["code"][field] += libproj[field] - db_bytes
        installs[-1][field] += libproj[field] - db_bytes

    rust = add("ours:program", args.rs_bin, "Rust program")
    empty = add("ours:empty", args.rs_empty, "empty Rust program")
    for key, label, binary, note in (
        ("ours", "proj-rs, Rust program", args.rs_bin,
         "proj-rs with its default features: static SQLite and proj.db inside the program, no TLS library"),
        ("ours_network", "proj-rs, Rust program, feature network", args.rs_network_bin,
         "proj-rs with the `network` feature: the HTTPS grid download, TLS from the system"),
    ):
        program = rust if binary == args.rs_bin else add(f"{key}:program", binary, "Rust program")
        if not contains(binary, args.rs_db):
            raise SystemExit(f"footprint: {binary} does not hold proj.db unchanged")
        libs = dependencies(binary, None)
        install(key, label, "ours",
                [{"name": "footprint-rs (without proj.db)", "bytes": program["bytes"] - db_bytes,
                  "stripped_bytes": program["stripped_bytes"] - db_bytes}],
                {"items": ["proj.db, inside the program"], "bytes": db_bytes, "stripped_bytes": db_bytes}, libs,
                note)
    return {"files": list(files.values()), "installs": installs, "embedded_db_bytes": db_bytes,
            "empty_rust_program": empty}


def parse_output(text):
    return {key: float(value) for key, value in (item.split("=") for item in text.split())}


def command(args, variant, mode):
    env = {"PATH": os.environ["PATH"], "PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY": "YES"}
    if variant == "ours":
        return [str(args.rs_bin), mode], env
    program, lib_dir, db = {
        "proj": (args.c_proj, args.proj_lib, args.proj_db),
        "proj_full": (args.c_proj_full, args.proj_full_lib, args.proj_full_db),
        "capi": (args.c_capi, args.capi_lib.parent, None),
    }[variant]
    env["LD_LIBRARY_PATH"] = str(lib_dir)
    if db:
        env["PROJ_DATA"] = str(db.parent)
    return [str(program), mode], env


def peak_rss(cmd, env):
    done = subprocess.run(cmd, env=env, text=True, capture_output=True, check=True)
    kb = int(re.search(r"vmhwm_kb=(\d+)", done.stderr).group(1))
    return kb, parse_output(done.stdout.strip())


def heap_peak(cmd, env, scratch):
    out = Path(scratch) / "massif.out"
    out.unlink(missing_ok=True)
    subprocess.run(
        ["valgrind", "--tool=massif", f"--massif-out-file={out}", "--peak-inaccuracy=0.5", "--time-unit=B", *cmd],
        env=env, check=True, capture_output=True,
    )
    return max(int(m.group(1)) for m in re.finditer(r"^mem_heap_B=(\d+)$", out.read_text(), re.M))


def measure_memory(args, scratch, massif):
    scenarios = []
    for mode, label in SCENARIOS:
        entry = {"id": mode, "label": label}
        reference = None
        for variant in VARIANTS:
            cmd, env = command(args, variant, mode)
            runs, result = [], None
            for _ in range(RUNS):
                kb, result = peak_rss(cmd, env)
                runs.append(kb)
            if reference is None:
                reference = result
            same = (result["ok"] == reference["ok"] and result["fail"] == reference["fail"]
                    and abs(result["sum"] - reference["sum"]) <= SUM_TOLERANCE * max(1.0, abs(reference["sum"])))
            if not same:
                raise SystemExit(f"footprint: {variant} {mode} gives {result}, PROJ gives {reference}")
            entry[variant] = {"peak_rss_kb": int(statistics.median(runs)), "runs_kb": sorted(runs)}
            if massif:
                entry[variant]["peak_heap_bytes"] = heap_peak(cmd, env, scratch)
        entry["ok"], entry["fail"] = int(reference["ok"]), int(reference["fail"])
        scenarios.append(entry)
        print(f"memory {mode}: " + ", ".join(f"{v} {entry[v]['peak_rss_kb']} KiB" for v in VARIANTS))
    return {"runs": RUNS, "points": 1_000_000, "statistic": "median", "scenarios": scenarios}


def versions(args):
    def first_line(cmd):
        return run(cmd, env={**os.environ, "LC_ALL": "C"}).splitlines()[0].strip()

    glibc = first_line(["ldd", "--version"])
    return {
        "rustc": first_line(["rustc", "-V"]),
        "cc": first_line(["cc", "--version"]),
        "valgrind": first_line(["valgrind", "--version"]) if shutil.which("valgrind") else None,
        "libc": glibc,
        "proj": "9.9.0",
    }


def cpu_model():
    for line in Path("/proc/cpuinfo").read_text().splitlines():
        if line.startswith("model name"):
            return line.split(":", 1)[1].strip()
    return platform.processor()


def main():
    parser = argparse.ArgumentParser()
    for name in ("proj-lib", "proj-db", "proj-full-lib", "proj-full-db", "capi-lib", "rs-db",
                 "c-proj", "c-proj-full", "c-capi", "rs-bin", "rs-network-bin", "rs-empty", "out"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--machine", default=platform.node())
    parser.add_argument("--no-massif", action="store_true")
    args = parser.parse_args()

    commit = run(["git", "rev-parse", "--short", "HEAD"], cwd=history.ROOT).strip()
    dirty = subprocess.run(["git", "diff", "--quiet", "HEAD", "--", "src"], cwd=history.ROOT).returncode != 0
    with tempfile.TemporaryDirectory() as scratch:
        sizes = measure_sizes(args, scratch)
        massif = not args.no_massif and shutil.which("valgrind") is not None
        memory = measure_memory(args, scratch, massif)
    result = {
        "date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "commit": commit,
        "dirty": dirty,
        "machine": args.machine,
        "cpu": cpu_model(),
        "kernel": platform.release(),
        "versions": versions(args),
        "rule": (
            "Everything `ldd` lists for the program or the library, except the C library and the "
            "unwinder (libc, libm, libdl, libpthread, librt, libutil, libgcc_s, the loader), "
            "counted the same way on both sides. Sizes are bytes on disk after `strip --strip-unneeded`."
        ),
        "size": sizes,
        "memory": memory,
    }
    args.out.write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8")
    print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
