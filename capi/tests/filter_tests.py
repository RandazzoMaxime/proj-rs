#!/usr/bin/env python3
"""Copy test_c_api.cpp without the TEST cases listed in excluded.txt.

usage: filter_tests.py SOURCE EXCLUDED OUTPUT

excluded.txt holds one case per line: `CApi.name` (the gtest name), then the reason after
a `#`. A case runs from its `TEST_F(` line to the first line that is exactly `}`. A name that
is not in the source is an error: the list must follow the file.
"""
import re
import sys


def main() -> int:
    source, excluded, output = sys.argv[1:4]
    names = set()
    for line in open(excluded, encoding="utf-8"):
        line = line.split("#", 1)[0].strip()
        if line.startswith("CApi."):
            names.add(line.split(".", 1)[1])
    lines = open(source, encoding="utf-8").read().split("\n")
    kept, skipping, removed = [], False, set()
    case = re.compile(r"^TEST(?:_F)?\((\w+),\s*(\w+)\)")
    for line in lines:
        if not skipping:
            found = case.match(line)
            if found and found.group(2) in names:
                skipping = True
                removed.add(found.group(2))
                continue
            kept.append(line)
        elif line == "}":
            skipping = False
    missing = names - removed
    if missing:
        print("not in the source: " + ", ".join(sorted(missing)), file=sys.stderr)
        return 1
    open(output, "w", encoding="utf-8").write("\n".join(kept))
    print(f"removed {len(removed)} cases", file=sys.stderr)
    return 0


sys.exit(main())
