#!/usr/bin/env python3
"""Compare the two GMT passes of tools/clients/gmt.sh.

Usage: gmt_compare.py WORKDIR BUILD GM REF_LIBDIR OURS_LIBDIR EXPECTED_TESTS PROJ_COUNT_SO

1. Every ctest entry must have an outcome in ctest-ref.log and in ctest-ours.log, EXPECTED_TESTS
   of them (`ctest -N`), and the same outcome on both sides; a test whose outcome differs goes
   through the solo reruns of 2., on its gmtest exit status. Each log is the first ctest run
   followed by its `--rerun-failed` runs: the last outcome of a test counts, and the tests a rerun
   changed are listed (ctest-<side>-1.log is the first run alone).
2. Every test directory (outputs-ref/ vs outputs-ours/; gmt.sh keeps the directory of passing
   tests too) must be on both sides, and every file in it must be the same: byte for byte, or
   once normalised (PostScript: the DSC date line and the temporary file names of the `%@GMT:`
   comments aside; netCDF: the ncdump text, the byte of the `grid_mapping` variable GMT leaves
   unset aside). A PostScript whose bytes still differ
   is rendered with GraphicsMagick (gm) and compared with the RMS limit of GMT's gmtest; under
   the limit it is not a failure, and it is listed apart.
   A file that differs, or that only one side wrote, is rerun alone once with each library
   (solo pair): under a loaded parallel run GMT does not always write the same files (a psxy at
   the end of a pipe can skip its layer). A solo pair that agrees, by the same rule, makes it
   GMT's noise under load. A pair that does not agree (or where a run did not write the file)
   is rerun once more with the reference library: if the two reference runs differ too, GMT
   is not deterministic there and no verdict on the library is possible (listed apart); if
   they agree, the difference is real and fails the comparison.
3. The calls each test made into libproj (the proj_count.c logs of both passes) must be the
   same: the same number of calls and points per function. A test whose calls differ goes
   through the same solo reruns (each solo run starts from a copy of the seed home directory, so
   a remote dataset it uses beyond the seed is downloaded and converted again, by that run alone).
Floors: fewer than MIN_TESTS outcomes or MIN_FILES compared files fails the comparison.

Not compared: the PDFs (Ghostscript writes its creation date in them; they are converted from
the PostScript that is compared), gmt.history and *.tag (timestamps), the test scripts.
"""

import collections
import filecmp
import os
import re
import shutil
import subprocess
import sys

RMS_LIMIT = 0.003
MIN_TESTS = 1100
MIN_FILES = 3000
SKIP_EXT = (".pdf", ".history", ".tag")
SKIP_FILES = {"gmt.history"}
PS_VARIABLE = re.compile(rb"^%%(CreationDate|ModDate):")
PS_TMP = re.compile(rb"/tmp/[^\s]+")
OUTCOME = re.compile(r"\s*\d+/\d+\s+Test\s+#\d+: (\S+)\s+\.*\s*\**(.+?)\s+[0-9.]+ sec")


def outcomes(path):
    result = {}
    with open(path, errors="replace") as f:
        for line in f:
            m = OUTCOME.match(line)
            if m:
                result[m.group(1)] = m.group(2).strip()
    return result


def gm_rms(gm, a, b):
    try:
        out = subprocess.run([gm, "compare", "-density", "200", "-metric", "rmse", a, b],
                             capture_output=True, text=True, timeout=300).stdout
    except subprocess.TimeoutExpired:
        return None
    m = re.search(r"Total: ([0-9.]+)", out)
    return float(m.group(1)) if m else None


def ps_body(path):
    """The lines of a PostScript file, the DSC date aside, and in the `%@GMT:` comments (the
    command line of each layer) the names of temporary files, which carry a process id or a
    random suffix (/tmp/GMT_symbol2143, /tmp/=tiled_1_PX_BoJlqc)."""
    with open(path, "rb") as f:
        return [PS_TMP.sub(b"/tmp/*", line) if line.startswith(b"%@GMT:") else line
                for line in f if not PS_VARIABLE.match(line)]


def ps_equal(a, b):
    return ps_body(a) == ps_body(b)


def nc_dump(path):
    """ncdump of a netCDF file, the data of its `grid_mapping` variable aside, or None.

    GMT writes `grid_mapping` as a scalar char that it never sets: its byte is whatever the
    memory held, and two runs with the same library differ there (seen in grdproject/units).
    """
    with open(path, "rb") as f:
        magic = f.read(4)
    if magic[:3] != b"CDF" and magic != b"\x89HDF":
        return None
    out = subprocess.run(["ncdump", path], capture_output=True, timeout=300)
    if out.returncode != 0:
        return None
    lines = out.stdout.split(b"\n")[1:]
    return [line for line in lines if not re.match(rb"\s*grid_mapping = ", line)]


def same(a, b, gm):
    """Whether two files written by the same test agree, and how (bytes, normalised, RMS)."""
    if filecmp.cmp(a, b, shallow=False):
        return True, "same bytes"
    if a.endswith(".ps"):
        if ps_equal(a, b):
            return True, "same bytes but the DSC date and temporary names"
        rms = gm_rms(gm, a, b)
        if rms is not None and rms <= RMS_LIMIT:
            return True, f"rms {rms}, bytes differ"
        return False, f"rms {rms}"
    da = nc_dump(a)
    if da is not None and da == nc_dump(b):
        return True, "same ncdump but the grid_mapping byte"
    return False, "differs"


def skip(filename):
    return (filename in SKIP_FILES or filename.endswith(SKIP_EXT)
            or filename.endswith(".sh") or filename == "gmtest.sh")


def walk(d):
    found = set()
    for base, dirs, files in os.walk(d):
        dirs[:] = [x for x in dirs if x != "CMakeFiles"]
        for f in files:
            found.add(os.path.relpath(os.path.join(base, f), d))
    return found


class Solo:
    """Reruns of one test alone, kept under WORKDIR/solo/. Each run starts from a copy of the
    seed home directory of gmt.sh (GMT_HOME_SEED, copied to GMT_HOME, the path of the passes) and
    has its own proj_count.c log.

    `verdict` compares one aspect of the test (a file, the gmtest status, the libproj calls)
    across runs: 'agree' when the solo pair agrees; 'gmt' when two runs with the same library
    differ, so GMT is not deterministic there and no verdict on the library is possible (with
    proj-rs, only when the test makes no libproj call in either pass); 'differ' when each
    library reproduces its own result and the two results differ.
    """

    def __init__(self, work, build, libdirs, preload):
        self.work, self.build, self.libdirs, self.preload = work, build, libdirs, preload
        self.done = {}

    def run(self, test, side):
        key = (test, side)
        if key not in self.done:
            lib = side.rstrip("2")
            slug = test[:-3].replace("/", "__")
            log = os.path.join(self.work, "solo", "logs", side, slug + ".log")
            os.makedirs(os.path.dirname(log), exist_ok=True)
            if os.path.exists(log):
                os.remove(log)
            home = os.environ["GMT_HOME"]
            shutil.rmtree(home, ignore_errors=True)
            shutil.copytree(os.environ["GMT_HOME_SEED"], home, symlinks=True)
            env = dict(os.environ)
            old = env.get("LD_LIBRARY_PATH")
            env["LD_LIBRARY_PATH"] = self.libdirs[lib] + (f":{old}" if old else "")
            env["LD_PRELOAD"] = self.preload
            env["PROJ_COUNT_LOG"] = log
            env["HOME"] = home
            status = subprocess.run(["bash", os.path.join(self.build, "test", "gmtest"), test],
                                    cwd=os.path.join(self.build, "test"), env=env,
                                    capture_output=True, timeout=1800).returncode
            fresh = os.path.join(self.build, test[:-3])
            kept = os.path.join(self.work, "solo", side, test[:-3])
            shutil.rmtree(kept, ignore_errors=True)
            if os.path.isdir(fresh):
                os.makedirs(os.path.dirname(kept), exist_ok=True)
                shutil.move(fresh, kept)
            calls = collections.Counter()
            if os.path.isfile(log):
                for c in proj_calls(log, self.build)[0].values():
                    calls.update(c)
            self.done[key] = {"dir": kept, "status": "Passed" if status == 0 else "Failed",
                              "calls": dict(calls), "log": log, "lib": lib}
        return self.done[key]

    def verdict(self, test, cmp, calls):
        ok, note = cmp("ref", "ours")
        if ok:
            return "agree", f"solo pair: {note}"
        ok2, note2 = cmp("ref", "ref2")
        if not ok2:
            return "gmt", f"solo pair: {note}; two reference runs: {note2}"
        ok3, note3 = cmp("ours", "ours2")
        notes = f"solo pair: {note}; two reference runs: {note2}; two proj-rs runs: {note3}"
        if not ok3 and not calls:
            return "gmt", f"{notes}; no libproj call in this test"
        return "differ", f"{notes}; libproj calls in the passes: {calls or 'none'}"

    def file(self, test, fn, gm, calls):
        def cmp(x, y):
            a = os.path.join(self.run(test, x)["dir"], fn)
            b = os.path.join(self.run(test, y)["dir"], fn)
            if not os.path.isfile(a) and not os.path.isfile(b):
                # Two runs of one library that both skip the file agree; a solo pair that does
                # not write it proves nothing about the two libraries.
                return x.rstrip("2") == y.rstrip("2"), "neither solo run wrote it"
            if not (os.path.isfile(a) and os.path.isfile(b)):
                return False, f"the {x if not os.path.isfile(a) else y} solo run did not write it"
            return same(a, b, gm)
        return self.verdict(test, cmp, calls)

    def outcome(self, test, calls):
        def cmp(x, y):
            a, b = self.run(test, x)["status"], self.run(test, y)["status"]
            return a == b, f"{a} vs {b}"
        return self.verdict(test, cmp, calls)

    def proj(self, test, calls):
        def cmp(x, y):
            a, b = self.run(test, x)["calls"], self.run(test, y)["calls"]
            return a == b, "same libproj calls" if a == b else f"{a} vs {b}"
        return self.verdict(test, cmp, calls)

    def check_libs(self):
        """Every process of the reruns mapped the libproj of its side."""
        bad = []
        maps = collections.Counter()
        for run in self.done.values():
            want = os.path.realpath(os.path.join(self.libdirs[run["lib"]], "libproj.so.25"))
            if not os.path.isfile(run["log"]):
                continue
            with open(run["log"], errors="replace") as f:
                for line in f:
                    parts = line.rstrip("\n").split("\t")
                    if parts[0] == "map":
                        maps[run["lib"]] += 1
                        if parts[4] != want:
                            bad.append(f"{run['lib']} rerun: pid {parts[1]} mapped {parts[4]}")
        for lib, libdir in self.libdirs.items():
            want = os.path.realpath(os.path.join(libdir, "libproj.so.25"))
            print(f"reruns {lib}: {maps[lib]} processes mapped a libproj, "
                  f"{'all ' + want if not bad else 'NOT ALL ' + want}")
        return bad


def proj_calls(path, build):
    """Per test directory, the summed calls/points per function of the proj_count.c log."""
    per_test = collections.defaultdict(collections.Counter)
    processes = collections.Counter()
    with open(path, errors="replace") as f:
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if parts[0] != "calls" or len(parts) < 6:
                continue
            cwd = os.path.relpath(parts[3], build) if parts[3].startswith(build) else parts[3]
            processes[cwd] += 1
            for item in parts[5].split():
                name, value = item.split("=")
                c, p = value.split("/")
                per_test[cwd][name + " calls"] += int(c)
                per_test[cwd][name + " points"] += int(p)
    return per_test, processes


def main():
    if len(sys.argv) != 8:
        sys.exit(__doc__.split("\n\n")[1])
    work, build, gm, ref_libdir, ours_libdir, expected, preload = sys.argv[1:8]
    expected = int(expected)
    failures = []

    ref = outcomes(os.path.join(work, "ctest-ref.log"))
    ours = outcomes(os.path.join(work, "ctest-ours.log"))
    tests = sorted(set(ref) | set(ours))
    outcome_diff = [t for t in tests if ref.get(t) != ours.get(t)]
    counts = collections.Counter(ref.values())
    print(f"tests: {len(ref)} ref, {len(ours)} ours, {expected} listed by ctest; "
          f"ref outcomes {dict(counts)}; outcomes that differ: {len(outcome_diff)}")
    if not (len(ref) == len(ours) == expected) or expected < MIN_TESTS:
        print(f"  floor: {expected} tests listed, {len(ref)}/{len(ours)} outcomes read "
              f"(at least {MIN_TESTS}, all of them)")
        failures.append("test floor")

    for side, final in (("ref", ref), ("ours", ours)):
        first_path = os.path.join(work, f"ctest-{side}-1.log")
        first = outcomes(first_path) if os.path.isfile(first_path) else final
        rerun = sorted(t for t in final if first.get(t) != final[t])
        print(f"{side}: {len(rerun)} tests failed the first run and changed outcome on a rerun "
              f"(ctest --rerun-failed, as GMT's CI)")
        for t in rerun:
            print(f"  {t}: {first.get(t)} then {final[t]}")

    calls = {}
    for side in ("ref", "ours"):
        calls[side] = proj_calls(os.path.join(work, f"projcount-{side}.log"), build)
    for side in ("ref", "ours"):
        per_test, processes = calls[side]
        total = collections.Counter()
        for c in per_test.values():
            total.update(c)
        summary = ", ".join(f"{k} {v}" for k, v in sorted(total.items()) if v)
        print(f"libproj calls, {side}: {len(per_test)} test directories, "
              f"{sum(processes.values())} processes; {summary}")
    solo = Solo(work, build, {"ref": ref_libdir, "ours": ours_libdir}, preload)
    real, nondet, gmt_nondet, ps_rms_only, nc_norm, one_dir, no_dir = [], [], [], [], [], [], []
    files_compared = ps_compared = 0

    def test_calls(t):
        a = {k: v for k, v in calls["ref"][0].get(t[:-3], {}).items() if v}
        b = {k: v for k, v in calls["ours"][0].get(t[:-3], {}).items() if v}
        if not a and not b:
            return ""
        return f"ref {sum(v for k, v in a.items() if k.endswith('calls'))}, " \
               f"ours {sum(v for k, v in b.items() if k.endswith('calls'))}"

    outcome_kinds = {"agree": [], "gmt": [], "differ": []}
    for t in outcome_diff:
        v, note = solo.outcome(t, test_calls(t))
        outcome_kinds[v].append(f"{t}: {ref.get(t)} vs {ours.get(t)} in the passes; {note}")
    for v, title in (("agree", "a solo pair agrees (GMT under load)"),
                     ("gmt", "two runs with the same library differ (GMT not deterministic)"),
                     ("differ", "real")):
        print(f"  outcomes that differ, {title}: {len(outcome_kinds[v])}")
        for line in outcome_kinds[v]:
            print(f"    {line}")
    if outcome_kinds["differ"]:
        failures.append("outcomes")

    def rerun_verdict(t, fn, why):
        v, note = solo.file(t, fn, gm, test_calls(t))
        entry = f"{t[:-3]}/{fn} (pass: {why}; {note})"
        {"agree": nondet, "gmt": gmt_nondet, "differ": real}[v].append(entry)

    for t in tests:
        rel = t[:-3]
        d_ref = os.path.join(work, "outputs-ref", rel)
        d_ours = os.path.join(work, "outputs-ours", rel)
        has_ref, has_ours = os.path.isdir(d_ref), os.path.isdir(d_ours)
        if not has_ref and not has_ours:
            no_dir.append(t)
            continue
        if has_ref != has_ours:
            one_dir.append(f"{rel} (only in {'ref' if has_ref else 'ours'})")
            continue
        for fn in sorted(walk(d_ref) | walk(d_ours)):
            if skip(os.path.basename(fn)):
                continue
            p_ref, p_ours = os.path.join(d_ref, fn), os.path.join(d_ours, fn)
            if not (os.path.isfile(p_ref) and os.path.isfile(p_ours)):
                side = "ref" if os.path.isfile(p_ref) else "ours"
                rerun_verdict(t, fn, f"written by the {side} pass only")
                continue
            files_compared += 1
            ps_compared += fn.endswith(".ps")
            ok, note = same(p_ref, p_ours, gm)
            if ok and note.startswith("rms"):
                v, solo_note = solo.file(t, fn, gm, test_calls(t))
                ps_rms_only.append(f"{rel}/{fn} ({note}; {solo_note})")
            elif ok and note.startswith("same ncdump"):
                nc_norm.append(f"{rel}/{fn}")
            elif not ok:
                rerun_verdict(t, fn, note)

    print(f"files compared: {files_compared} ({ps_compared} PostScript)")
    print(f"tests without a directory on either side (disabled by GMT_DISABLE_TEST): {len(no_dir)}")
    for t in no_dir:
        print(f"  {t}")
    print(f"test directories on one side only: {len(one_dir)}")
    for f in one_dir:
        print(f"  {f}")
    print(f"netCDF files that differ only by the grid_mapping byte GMT leaves unset: {len(nc_norm)}")
    print(f"PostScript with different bytes but an RMS under the limit: {len(ps_rms_only)}")
    for f in ps_rms_only:
        print(f"  {f}")
    print(f"differences of the pass that a solo pair resolves (GMT under load): {len(nondet)}")
    for f in nondet:
        print(f"  {f}")
    print(f"differences where two runs with the same library differ (GMT not deterministic, "
          f"no verdict on the library): {len(gmt_nondet)}")
    for f in gmt_nondet:
        print(f"  {f}")
    print(f"real differences (each library reproduces its own file): {len(real)}")
    for f in real:
        print(f"  {f}")
    if files_compared < MIN_FILES:
        print(f"  floor: {files_compared} files compared, at least {MIN_FILES}")
        failures.append("file floor")
    if one_dir:
        failures.append("directories")
    if real:
        failures.append("files")

    calls_diff = [d for d in sorted(set(calls["ref"][0]) | set(calls["ours"][0]))
                  if calls["ref"][0].get(d) != calls["ours"][0].get(d)]
    calls_kinds = {"agree": [], "gmt": [], "differ": []}
    for d in calls_diff:
        a, b = calls["ref"][0].get(d, {}), calls["ours"][0].get(d, {})
        keys = sorted(k for k in set(a) | set(b) if a.get(k, 0) != b.get(k, 0))
        what = ", ".join(f"{k} {a.get(k, 0)} vs {b.get(k, 0)}" for k in keys)
        if os.path.isfile(os.path.join(build, "..", "src", d + ".sh")):
            v, note = solo.proj(d + ".sh", test_calls(d + ".sh"))
        else:
            v, note = "differ", "not a test directory"
        calls_kinds[v].append(f"{d}: {what} in the passes; {note}")
    print(f"test directories whose libproj calls differ in the passes: {len(calls_diff)}")
    for v, title in (("agree", "a solo pair agrees (which test downloads and converts a remote "
                               "dataset first depends on the schedule)"),
                     ("gmt", "two runs with the same library differ"),
                     ("differ", "real")):
        print(f"  {title}: {len(calls_kinds[v])}")
        for line in calls_kinds[v]:
            print(f"    {line}")
    if calls_kinds["differ"]:
        failures.append("libproj calls")

    bad = solo.check_libs()
    for b in bad:
        print(f"  {b}")
    if bad:
        failures.append("rerun libraries")

    if failures:
        print(f"FAILED: {', '.join(failures)}")
        sys.exit(1)
    print("every test has the same outcome, every file the same content and every test the same "
          "libproj calls with both libraries.")


if __name__ == "__main__":
    main()
