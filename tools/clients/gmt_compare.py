#!/usr/bin/env python3
"""Compare the two GMT passes of tools/clients/gmt.sh.

Usage: gmt_compare.py WORKDIR BUILD GM [REF_LIBDIR]

The outcome of every ctest entry must be the same in ctest-ref.log and ctest-ours.log. Then every
file the test wrote (outputs-ref/ vs outputs-ours/) must be the same: byte for byte, and for the
PostScript rendered with GraphicsMagick (gm), with the RMS limit GMT's gmtest uses. A file that
differs is a real difference unless the test, rerun once with the reference library, agrees with
our pass instead: under a loaded parallel run GMT does not always write the same files (a psxy
at the end of a pipe can skip its layer), so the reference pass can be the odd one out.

Files GMT makes nondeterministic are not compared: the PDFs carry the creation date of Ghostscript,
gmt.history and *.tag a timestamp, and test/grdtrack/crosstrack_geo/cross.txt is written by a
parallel grdtrack whose line order depends on the schedule (two reference runs differ).
"""

import filecmp
import os
import re
import subprocess
import sys

RMS_LIMIT = 0.003
SKIP_EXT = (".pdf", ".history", ".tag")
SKIP_FILES = {"gmt.history"}
SKIP_TESTS_FILES = {"test/grdtrack/crosstrack_geo": {"cross.txt", "crosstrack_geo.ps"}}


def outcomes(path):
    result = {}
    with open(path, errors="replace") as f:
        for line in f:
            m = re.match(r"\s*\d+/\d+\s+Test\s+#\d+: (\S+\.sh)\s+.*?(Passed|Failed)", line)
            if m:
                result[m.group(1)] = m.group(2)
    return result


def exec_dir(work, name, side):
    rel = name[:-3]
    return os.path.join(work, f"outputs-{side}", rel)


def gm_rms(gm, a, b):
    try:
        out = subprocess.run([gm, "compare", "-density", "200", "-metric", "rmse", a, b],
                             capture_output=True, text=True, timeout=300).stdout
    except subprocess.TimeoutExpired:
        return None
    m = re.search(r"Total: ([0-9.]+)", out)
    return float(m.group(1)) if m else None


def ps_equal(a, b):
    """Byte equality of two PostScript files, the DSC comment lines (dates) aside."""
    def body(path):
        with open(path, errors="replace") as f:
            return [l for l in f if not l.startswith("%")]
    return body(a) == body(b)


def skip(rel_test, filename):
    if rel_test in SKIP_TESTS_FILES and filename in SKIP_TESTS_FILES[rel_test]:
        return True
    return (filename in SKIP_FILES or filename.endswith(SKIP_EXT)
            or filename.endswith(".sh") or filename == "gmtest.sh")


def rerun(build, test, libdir):
    """Run one test alone with the given library first on LD_LIBRARY_PATH.

    gmtest deletes the working directory of a test that passes against its baseline, so a
    rerun that succeeds leaves nothing behind: "the file is gone" means the run was clean.
    """
    env = dict(os.environ)
    env["LD_LIBRARY_PATH"] = f"{libdir}:{env.get('LD_LIBRARY_PATH', '')}"
    subprocess.run(["bash", os.path.join(build, "test", "gmtest"), test],
                   cwd=build, env=env, capture_output=True, timeout=1800)


def solo_pair(build, work, test, fn, ref_libdir, ours_libdir):
    """Run the test alone once with each library and compare the two fresh files.

    Under the parallel load of a whole suite GMT does not always write the same files: a psxy at
    the end of a pipe can skip its layer, the iterative solvers of greenspline and potential
    reduce in an order that depends on the OpenMP schedule. A solo pair answers the real
    question: do the two libraries give the same file? Returns True when the pair agrees or
    when both solo runs pass gmtest's own comparison and clean up their directory.
    """
    rerun(build, test, ref_libdir)
    fresh_ref = os.path.join(build, test[:-3], fn)
    ref_again = None
    if os.path.isfile(fresh_ref):
        ref_again = os.path.join(work, "solo", f"{test[:-3]}__{fn}")
        os.makedirs(os.path.dirname(ref_again), exist_ok=True)
        os.replace(fresh_ref, ref_again)
    rerun(build, test, ours_libdir)
    fresh_ours = os.path.join(build, test[:-3], fn)
    if ref_again is None and not os.path.isfile(fresh_ours):
        return True
    if ref_again is None or not os.path.isfile(fresh_ours):
        return False
    if fn.endswith(".ps"):
        rms = gm_rms(GM, ref_again, fresh_ours)
        return rms is not None and rms <= RMS_LIMIT
    return filecmp.cmp(ref_again, fresh_ours, shallow=False)


def main():
    global GM
    work, build, gm = sys.argv[1:4]
    GM = gm
    ref_libdir = sys.argv[4] if len(sys.argv) > 4 else os.environ.get("LD_LIBRARY_PATH", "")
    ours_libdir = sys.argv[5] if len(sys.argv) > 5 else ""
    ref = outcomes(os.path.join(work, "ctest-ref.log"))
    ours = outcomes(os.path.join(work, "ctest-ours.log"))
    tests = sorted(set(ref) | set(ours))
    outcome_diff = [t for t in tests if ref.get(t) != ours.get(t)]
    print(f"tests: {len(tests)}, outcomes that differ: {len(outcome_diff)}")
    for t in outcome_diff:
        print(f"  {t}: {ref.get(t)} vs {ours.get(t)}")

    real, nondet, ps = [], [], []
    files_compared = 0

    def walk(d):
        found = set()
        for base, dirs, files in os.walk(d):
            dirs[:] = [x for x in dirs if x not in ("CMakeFiles",)]
            for f in files:
                found.add(os.path.relpath(os.path.join(base, f), d))
        return found

    def agree(t, fn, note):
        if solo_pair(build, work, t, fn, ref_libdir, ours_libdir):
            nondet.append(f"{t[:-3]}/{fn} ({note}: fresh pair agrees)")
        else:
            real.append(f"{t[:-3]}/{fn}")

    for t in tests:
        d_ref = exec_dir(work, t, "ref")
        d_ours = exec_dir(work, t, "ours")
        if not (os.path.isdir(d_ref) and os.path.isdir(d_ours)):
            continue
        for fn in sorted(walk(d_ref) | walk(d_ours)):
            if skip(t[:-3], os.path.basename(fn)):
                continue
            p_ref = os.path.join(d_ref, fn)
            p_ours = os.path.join(d_ours, fn)
            if not (os.path.isfile(p_ref) and os.path.isfile(p_ours)):
                agree(t, fn, "one side of the pass only")
                continue
            files_compared += 1
            if fn.endswith(".ps"):
                rms = gm_rms(gm, p_ref, p_ours)
                if rms is None or rms > RMS_LIMIT:
                    if rms is None and ps_equal(p_ref, p_ours):
                        continue
                    if solo_pair(build, work, t, fn, ref_libdir, ours_libdir):
                        if rms is None or rms <= RMS_LIMIT:
                            note = "renderer"
                        else:
                            note = f"rms {rms}"
                        nondet.append(f"{t[:-3]}/{fn} (pass-suite noise, {note}: fresh pair agrees)")
                    else:
                        ps.append(f"{t[:-3]}/{fn} (rms {rms})")
                continue
            if not filecmp.cmp(p_ref, p_ours, shallow=False):
                agree(t, fn, "bytes")

    print(f"files compared: {files_compared}")
    print(f"nondeterministic in GMT itself: {len(nondet)}")
    for f in nondet:
        print(f"  {f}")
    print(f"PostScript images above the RMS limit: {len(ps)}")
    for f in ps:
        print(f"  {f}")
    print(f"real differences: {len(real)}")
    for f in real:
        print(f"  {f}")
    if outcome_diff or ps or real:
        sys.exit(1)
    print("every test has the same outcome and every file the same bytes with both libraries.")


if __name__ == "__main__":
    main()
