#!/usr/bin/env python3
"""Compare two JUnit reports of the same test suite: `compare_junit.py <reference.xml> <ours.xml>`.

Prints the tests that pass with the reference and fail with proj-rs, grouped by cause, and the
tests the reference ran that proj-rs did not (a module that fails to import runs none of its
tests). Writes the full list next to <ours.xml>, as <ours.xml>.regressions.tsv. Standard library only.
"""
import collections
import re
import sys
import xml.etree.ElementTree as ET


def load(path):
    out = {}
    for case in ET.parse(path).iter("testcase"):
        name = case.get("classname", "") + "::" + case.get("name", "")
        state, message = "pass", ""
        for kind in ("failure", "error"):
            element = case.find(kind)
            if element is not None:
                state = "fail"
                message = ((element.get("message") or "").strip().splitlines() or [""])[0]
        if case.find("skipped") is not None:
            state = "skip"
        out[name] = (state, message)
    return out


def cause(message):
    """The kind of gap a failure message points to, as the libproj API sees it."""
    rules = [
        (r"string: \{|projjson", "PROJJSON : un objet qui n'est pas un CRS n'est pas importé"),
        (r"^KeyError: \d+$", "proj_get_type : un CRS rendu là où PROJ rend une opération"),
        (r"\+init=", "proj_create : +init=<fichier>:<clé> non lu"),
        (r"urn:ogc:def:ensemble", "proj_create : URN d'ensemble de datums"),
        (r"urn:ogc:def:coordinateOperation", "proj_create : URN d'opération"),
        (r"proj_create_crs_to_crs_from_pj", "proj_create_crs_to_crs_from_pj échoue"),
        (r"normalize transformation", "proj_normalize_for_visualization échoue"),
        (r"NoneType", "NULL rendu là où PROJ rend une valeur"),
    ]
    for pattern, label in rules:
        if re.search(pattern, message, re.IGNORECASE):
            return label
    return "autre"


def main():
    if len(sys.argv) != 3:
        sys.exit("usage: compare_junit.py <reference.xml> <ours.xml>")
    reference, ours = load(sys.argv[1]), load(sys.argv[2])
    passed = lambda results: sum(state == "pass" for state, _ in results.values())
    regressions = {
        name: message
        for name, (state, message) in ours.items()
        if state == "fail" and reference.get(name, ("?",))[0] == "pass"
    }
    gains = [n for n, (s, _) in ours.items() if s == "pass" and reference.get(n, ("?",))[0] == "fail"]
    not_run = [n for n, (s, _) in reference.items() if s == "pass" and n not in ours]
    print(f"reference: {passed(reference)} passed | proj-rs: {passed(ours)} passed")
    print(f"pass with the reference, fail with proj-rs: {len(regressions)}")
    print(f"pass with the reference, not run with proj-rs: {len(not_run)}")
    print(f"pass with proj-rs, fail with the reference: {len(gains)}")
    print()
    for label, count in collections.Counter(cause(m) for m in regressions.values()).most_common():
        print(f"{count:5d}  {label}")
    with open(sys.argv[2] + ".regressions.tsv", "w", encoding="utf-8") as out:
        for name in sorted(regressions):
            out.write(f"{name}\t{cause(regressions[name])}\t{regressions[name]}\n")


if __name__ == "__main__":
    main()
