#!/usr/bin/env python3
"""Install the baseline PostScript images of a GMT source tree (tools/clients/gmt.sh).

Usage: gmt_baselines.py SRC BASELINE_DIR

GMT keeps its baselines in a DVC remote (DAGsHub), which refuses anonymous reads (HTTP 401). Each
release also publishes them as `baseline-images.zip` (workflow release-baseline-images.yml: a
`dvc pull` of the tag, then test/baseline/*, doc/scripts/images/*.ps and doc/examples/images/*.ps
under test/, scripts/ and examples/). BASELINE_DIR is that archive unpacked. Every `*.dvc` file of
SRC names the md5 of its output (a file) or of its directory listing (`<md5>.dir`, the md5 of the
JSON list of {"md5", "relpath"} that DVC writes); each output is checked against it, then copied to
the place of SRC that gmtest reads. Any mismatch, missing output or `.dvc` left without its output
fails. Standard library only.
"""
import hashlib
import json
import os
import re
import shutil
import sys

PLACES = {"test/baseline": "test", "doc/scripts/images": "scripts", "doc/examples/images": "examples"}


def md5(path):
    h = hashlib.md5()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def dir_md5(path):
    entries = []
    for base, _, files in os.walk(path):
        for name in files:
            full = os.path.join(base, name)
            entries.append({"md5": md5(full), "relpath": os.path.relpath(full, path).replace(os.sep, "/")})
    entries.sort(key=lambda e: e["relpath"])
    return hashlib.md5(json.dumps(entries, sort_keys=True).encode()).hexdigest() + ".dir"


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__.split("\n\n")[1])
    src, archive = sys.argv[1:]
    errors, files = [], 0
    dvcs = [os.path.relpath(os.path.join(b, f), src) for b, _, fs in os.walk(src)
            for f in fs if f.endswith(".dvc")]
    for dvc in sorted(dvcs):
        place = os.path.dirname(dvc)
        if place not in PLACES:
            errors.append(f"{dvc}: not in {', '.join(PLACES)}")
            continue
        text = open(os.path.join(src, dvc)).read()
        want = re.search(r"md5: (\S+)", text).group(1)
        out = re.search(r"path: (\S+)", text).group(1)
        got_path = os.path.join(archive, PLACES[place], out)
        if not os.path.exists(got_path):
            errors.append(f"{dvc}: {out} not in the archive")
            continue
        got = dir_md5(got_path) if want.endswith(".dir") else md5(got_path)
        if got != want:
            errors.append(f"{dvc}: md5 {got}, the .dvc says {want}")
            continue
        dest = os.path.join(src, place, out)
        if os.path.isdir(got_path):
            shutil.rmtree(dest, ignore_errors=True)
            shutil.copytree(got_path, dest)
            files += sum(len(fs) for _, _, fs in os.walk(dest))
        else:
            shutil.copyfile(got_path, dest)
            files += 1
    for e in errors:
        print(e, file=sys.stderr)
    if errors or not dvcs:
        sys.exit(f"baselines: {len(errors)} errors in {len(dvcs)} .dvc files")
    print(f"baselines: {len(dvcs)} .dvc outputs match their md5, {files} files installed")


if __name__ == "__main__":
    main()
