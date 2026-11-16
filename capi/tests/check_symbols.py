#!/usr/bin/env python3
"""Check that a library exports every function and variable of proj.h.

usage: check_symbols.py PROJ_H LIBRARY

The names are read from the declarations of proj.h (a `PROJ_DLL` function or variable), and the
exported symbols from `nm -D` (a static library: `nm`). A missing name is an error; an exported
`proj_*` or `pj_*` name that proj.h does not declare is an error too.
"""
import re
import subprocess
import sys


def declared(header: str) -> set[str]:
    text = open(header, encoding="utf-8").read()
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    names = set(re.findall(r"PROJ_DLL\s*\**\s*(\w+)\s*\(", text))
    names |= set(re.findall(r"\)\s*PROJ_DLL\s*\**\s*(\w+)\s*\(", text))
    names |= set(re.findall(r"extern char const PROJ_DLL (\w+)\[\]", text))
    names -= {"__attribute__", "__declspec"}
    return names


def exported(library: str) -> set[str]:
    command = ["nm", "-g", "--defined-only"]
    if library.endswith((".so", ".dylib")) or ".so." in library:
        command = ["nm", "-D", "--defined-only"]
    output = subprocess.run(command + [library], check=True, capture_output=True, text=True).stdout
    names = set()
    for line in output.splitlines():
        parts = line.split()
        if len(parts) >= 3 and parts[1] in "TDBRtdbr":
            names.add(parts[2].lstrip("_") if parts[2].startswith("__") else parts[2])
    return names


def main() -> int:
    header, library = sys.argv[1:3]
    want = declared(header)
    have = {name for name in exported(library) if name.startswith(("proj_", "pj_"))}
    missing = sorted(want - have)
    extra = sorted(have - want)
    print(f"proj.h declares {len(want)} names; the library exports {len(have)} proj_*/pj_* names")
    for name in missing:
        print(f"missing: {name}")
    for name in extra:
        print(f"not in proj.h: {name}")
    return 1 if missing or extra else 0


sys.exit(main())
