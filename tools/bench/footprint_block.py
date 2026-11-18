#!/usr/bin/env python3
"""Rewrite the README "Size and memory" block from bench/footprint.json.

The block between `<!-- bench-footprint:start -->` and `<!-- bench-footprint:end -->` holds the two
cards drawn by cards.py, a summary and the tables, all built from the file, so the README shows no
number that bench/footprint.sh did not measure. Standard library only.
"""
import json

import history

FOOTPRINT = history.ROOT / "bench" / "footprint.json"
MIB = 1024 * 1024

ROWS = {
    "proj": ("PROJ 9.9.0", "C program + libproj, TIFF and curl off"),
    "proj_full": ("PROJ 9.9.0", "C program + libproj, TIFF and curl on"),
    "capi": ("proj-rs as libproj", "same C program + libproj of capi/"),
    "ours": ("proj-rs", "Rust program, proj.db inside"),
    "ours_network": ("proj-rs, feature network", "Rust program, HTTPS grid download"),
}
VARIANT_NAMES = {
    "proj": "PROJ 9.9.0",
    "proj_full": "PROJ 9.9.0, TIFF and curl",
    "capi": "proj-rs C library",
    "ours": "proj-rs (Rust)",
}


def mib(value):
    return f"{value / MIB:.1f} MiB"


def load():
    return json.loads(FOOTPRINT.read_text(encoding="utf-8"))


def relative(ours, theirs, noun):
    """`ours` against `theirs`, as the README says it."""
    ratio = theirs / ours
    if ratio >= 1.05:
        return f"{ratio:.1f}× less {noun}"
    if ratio <= 1 / 1.05:
        return f"{1 / ratio:.1f}× more {noun}"
    return f"about the same {noun}"


def smaller_than(ours, theirs):
    ratio = theirs / ours
    if ratio >= 1.05:
        return f"{ratio:.1f}× smaller than"
    if ratio <= 1 / 1.05:
        return f"{1 / ratio:.1f}× larger than"
    return "about the size of"


def named_bytes(data, install, prefixes):
    sizes = {f["name"]: f["stripped_bytes"] for f in data["size"]["files"]}
    names = [n for n in install["parts"]["libs"]["items"] if n.startswith(prefixes)]
    return sum(sizes[n] for n in names) if names else None


def size_table(data):
    lines = [
        "| Install | Program and library | proj.db | Other shared libraries | of which OpenSSL | "
        "of which libcurl | Total, stripped | Total, as built |",
        "|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for install in data["size"]["installs"]:
        first, second = ROWS[install["id"]]
        parts = install["parts"]
        ssl = named_bytes(data, install, ("libssl", "libcrypto"))
        curl = named_bytes(data, install, ("libcurl",))
        lines.append(
            f"| {first}, {second} | {mib(parts['code']['stripped_bytes'])} | {mib(parts['db']['stripped_bytes'])} "
            f"| {mib(parts['libs']['stripped_bytes'])} | {'none' if ssl is None else mib(ssl)} "
            f"| {'none' if curl is None else mib(curl)} | **{mib(install['stripped_bytes'])}** "
            f"| {mib(install['bytes'])} |"
        )
    return "\n".join(lines)


def libs_text(data):
    lines = []
    for install in data["size"]["installs"]:
        libs = install["parts"]["libs"]["items"]
        names = ", ".join(f"`{n}`" for n in libs) if libs else "none"
        lines.append(f"- {ROWS[install['id']][0]}, {ROWS[install['id']][1]}: {names}.")
    return "\n".join(lines)


def memory_table(data, field, scale, unit, title):
    key_cols = [v for v in VARIANT_NAMES if all(v in s for s in data["memory"]["scenarios"])]
    lines = [
        f"| {title} | " + " | ".join(VARIANT_NAMES[v] for v in key_cols) + " |",
        "|---|" + "---:|" * len(key_cols),
    ]
    for s in data["memory"]["scenarios"]:
        cells = []
        for v in key_cols:
            value = s[v].get(field)
            cells.append("n/a" if value is None else f"{value / scale:.1f} {unit}")
        lines.append(f"| {s['label']} | " + " | ".join(cells) + " |")
    return "\n".join(lines)


def block(data):
    installs = {i["id"]: i for i in data["size"]["installs"]}
    ours, proj, full = installs["ours"], installs["proj"], installs["proj_full"]
    scenarios = {s["id"]: s for s in data["memory"]["scenarios"]}
    empty_rust = data["size"]["empty_rust_program"]
    program = next(f for f in data["size"]["files"] if f["name"] == "footprint-rs")
    db = data["size"]["embedded_db_bytes"]
    created, points, array, multi = (scenarios[k] for k in ("create", "points", "array", "multi"))
    rss = lambda s, v: s[v]["peak_rss_kb"]
    network_text = ""
    if "ours_network" in installs:
        net = installs["ours_network"]
        ssl = named_bytes(data, net, ("libssl", "libcrypto"))
        network_text = (
            f"With the `network` feature (the HTTPS grid download, TLS from the system) the program is "
            f"{mib(net['stripped_bytes'])}"
            + (f", of which {mib(ssl)} is OpenSSL" if ssl else "")
            + f": {smaller_than(net['stripped_bytes'], full['stripped_bytes'])} the PROJ with curl. "
        )
    lines = [
        "The same conversion, EPSG:4326 to EPSG:2154, measured on the same machine "
        "(Linux x86-64, glibc), proj-rs against PROJ 9.9.0.",
        "",
        '<img src="bench/cards/size.png" alt="Size on disk of a program that converts EPSG:4326 to 2154: '
        'proj-rs next to PROJ 9.9.0, minimal and with TIFF and curl" width="100%">',
        "",
        '<img src="bench/cards/memory.png" alt="Peak resident memory of five scenarios, proj-rs next to '
        'PROJ 9.9.0" width="100%">',
        "",
        f"On disk, a Rust program with proj-rs is {mib(ours['stripped_bytes'])} with `proj.db` included "
        f"({mib(db)} of it) and no library other than the C library: "
        f"{smaller_than(ours['stripped_bytes'], proj['stripped_bytes'])} a C program with PROJ 9.9.0 built "
        f"as in the speed comparison ({mib(proj['stripped_bytes'])}), and "
        f"{smaller_than(ours['stripped_bytes'], full['stripped_bytes'])} one with a PROJ built with TIFF "
        f"and curl ({mib(full['stripped_bytes'])}). {network_text}"
        f"The code of proj-rs alone, `proj.db` left out, "
        f"is {mib(ours['parts']['code']['stripped_bytes'])}; an empty Rust program is "
        f"{mib(empty_rust['stripped_bytes'])}.",
        "",
        f"In memory, creating the transform peaks at {rss(created, 'ours') / 1024:.1f} MiB for proj-rs and "
        f"{rss(created, 'proj') / 1024:.1f} MiB for PROJ "
        f"({relative(rss(created, 'ours'), rss(created, 'proj'), 'memory')}). With a million points one at a time "
        f"it is {rss(points, 'ours') / 1024:.1f} MiB and {rss(points, 'proj') / 1024:.1f} MiB; as one array, "
        f"{rss(array, 'ours') / 1024:.1f} MiB and {rss(array, 'proj') / 1024:.1f} MiB, "
        f"the array of points itself being {data['memory']['points'] * 16 / MIB:.1f} MiB; with several candidate operations (EPSG:4267 to 4326), "
        f"{rss(multi, 'ours') / 1024:.1f} MiB and {rss(multi, 'proj') / 1024:.1f} MiB.",
        "",
        "<details>",
        "<summary><b>Size and memory, component by component</b></summary>",
        "",
        "<br>",
        "",
        "Size on disk, after `strip --strip-unneeded`. A program is counted with every shared "
        f"library `ldd` lists for it, except the C library and the unwinder (libc, libm, libdl, libpthread, "
        f"librt, libutil, libgcc_s), on both sides. {history.day_time(data['date'])} UTC, commit "
        f"`{data['commit']}`.",
        "",
        size_table(data),
        "",
        "OpenSSL is `libssl` plus `libcrypto`, already inside the third column. proj-rs links it only "
        "with the `network` feature, for its HTTPS grid download, and takes it from the system "
        "(`native-tls`); PROJ pulls it through libcurl, when built with curl, together with the "
        "other libraries of curl (`libnghttp2`, `libssh`, `libgssapi_krb5`, `libldap`, `libgnutls` and more), "
        "which count in the third column but not in the libcurl column.",
        "",
        "Shared libraries counted in the third column:",
        "",
        libs_text(data),
        "",
        f"The Rust program is {mib(program['stripped_bytes'])} stripped, of which {mib(db)} is the "
        f"embedded `proj.db`; the empty Rust program of the same toolchain is "
        f"{mib(empty_rust['stripped_bytes'])}. The code of proj-rs thus adds "
        f"{mib(ours['parts']['code']['stripped_bytes'] - empty_rust['stripped_bytes'])}, SQLite and the "
        "TIFF reader included.",
        "",
        memory_table(data, "peak_rss_kb", 1024, "MiB", "Peak resident memory"),
        "",
        memory_table(data, "peak_heap_bytes", MIB, "MiB", "Peak heap (valgrind massif)"),
        "",
        f"Median of {data['memory']['runs']} runs for the resident memory (`VmHWM`, read by the program "
        "itself); the heap is one massif run. Each process runs one scenario on the same "
        f"{data['memory']['points']:,} points, and its output is checked against PROJ's. Every "
        "process starts with its own loader and libraries, so the first row is the floor. "
        "What exactly is measured, and how to rerun it: [docs/BENCHMARK.md](docs/BENCHMARK.md).",
        "",
        "</details>",
    ]
    return "\n".join(lines)


def main():
    history.write_block("bench-footprint", block(load()))


if __name__ == "__main__":
    main()
