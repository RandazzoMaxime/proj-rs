#!/usr/bin/env python3
"""List the cases of test_c_api.cpp that call the C++ API of PROJ, from a linker log.

usage: find_cpp_cases.py LINK_LOG

Link `test_c_api.cpp` (unfiltered) against libproj of proj-rs and give the output of `ld` to
this script. A case that references a symbol of `osgeo::proj::` needs the C++ library, which
proj-rs does not have: it goes into excluded.txt. A reference to a C function (`proj_*`) is a
missing function of the C API, and is not listed here.
"""
import re
import sys


def main() -> int:
    current = None
    cases = {}
    for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
        found = re.search(r"in function `\(anonymous namespace\)::(\w+)_Test::TestBody\(\)'", line)
        if found:
            current = found.group(1)
            continue
        if "in function" in line:
            current = None
            continue
        symbol = re.search(r"undefined reference to `(osgeo::[^']*|typeinfo for osgeo[^']*)'", line)
        if symbol and current:
            cases.setdefault(current, symbol.group(1).split("(")[0])
    for name in sorted(cases):
        print(f"{name}\t{cases[name]}")
    return 0


sys.exit(main())
