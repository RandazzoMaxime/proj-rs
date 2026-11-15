#!/usr/bin/env python3
"""Enregistre la sortie de PROJ 9.9.0 pour chaque invocation de tests/parity/cases.jsonl.

Écrit tests/parity/expected.jsonl : une ligne par invocation, avec le code de
sortie et la sortie standard ligne à ligne (la sortie d'erreur est ignorée).
Stdlib seulement. Appelé par record.sh.
"""

import argparse
import json
from pathlib import Path

from common import ROOT
from reference import Reference


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--reference-dir", required=True)
    parser.add_argument("--cases", default=str(ROOT / "tests/parity/cases.jsonl"))
    parser.add_argument("--out", default=str(ROOT / "tests/parity/expected.jsonl"))
    parser.add_argument("--grids-dir", help="dossier des grilles (corpus avec grilles)")
    options = parser.parse_args()
    reference = Reference(options.reference_dir, options.grids_dir)
    count = 0
    with open(options.cases, encoding="utf-8") as cases, \
            open(options.out, "w", encoding="utf-8") as out:
        for line in cases:
            case = json.loads(line)
            code, stdout = reference.run(case["tool"], case["args"], case["input"])
            out.write(json.dumps({"id": case["id"], "rc": code, "out": stdout},
                                 ensure_ascii=True) + "\n")
            count += 1
    print(f"{count} invocations recorded from {options.reference_dir} -> {options.out}")
    print(Path(options.out).stat().st_size, "bytes")


if __name__ == "__main__":
    main()
