#!/usr/bin/env python3
"""Différentiel aléatoire contre PROJ 9.9.0 (docs/FUZZ.md).

Tire des invocations `cs2cs`, `proj` et `cct` d'une graine : paires de CRS de la `proj.db`, points,
options, puis projections, chaînes PROJ et pipelines aux paramètres tirés au hasard. Enregistre la
sortie de la référence, lance `cargo test --test parity parity_random_against_proj_9_9_0`, qui
compare avec les seuils et le comparateur de tests/parity.rs, et écrit le rapport. La même graine
avec les mêmes options redonne exactement le même tirage (`meta.json` garde les options).

Stdlib seulement. La référence est lue dans --reference-dir (défaut : PROJ_REFERENCE_DIR, sinon
target/proj-reference), avec `build/bin` et `build/data`.
"""

import argparse
import hashlib
import json
import os
import random
import re
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import generate
from catalog import Catalog
from common import ROOT
from families_random import fam_random_cct, fam_random_cs2cs, fam_random_proj
from generator import Generator
from reference import Reference


def draw(options, reference_dir):
    catalog = Catalog(Path(reference_dir) / "build/data/proj.db")
    gen = Generator(catalog, Reference(reference_dir), random.Random(options.seed),
                    points=options.points, world=options.world)
    generate.build(gen, scale=options.scale)
    fam_random_proj(gen, options.random, points=options.points)
    fam_random_cs2cs(gen, options.random, points=options.points,
                     named_datums=options.named_datums)
    fam_random_cct(gen, options.random, points=options.points - 2)
    return gen


def record(reference, cases, jobs):
    def run(case):
        code, out = reference.run(case["tool"], case["args"], case["input"])
        return {"id": case["id"], "rc": code, "out": out}
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        return list(pool.map(run, cases))


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--seed", type=int, help="graine du tirage (défaut : l'heure, affichée)")
    parser.add_argument("--scale", type=int, default=3,
                        help="échelle des familles du corpus (5 000 conversions environ par unité)")
    parser.add_argument("--random", type=int, default=600,
                        help="invocations par famille aléatoire (proj, cs2cs, cct)")
    parser.add_argument("--points", type=int, default=8, help="points par invocation")
    parser.add_argument("--world", type=float, default=0.15,
                        help="part des points tirés dans le monde entier")
    parser.add_argument("--named-datums", action="store_true",
                        help="cs2cs aléatoire avec des datums nommés (+datum=NAD27…) : écarts connus")
    parser.add_argument("--reference-dir", default=os.environ.get(
        "PROJ_REFERENCE_DIR", str(ROOT / "target/proj-reference")))
    parser.add_argument("--out", help="dossier de travail (défaut : target/parity-random/<graine>)")
    parser.add_argument("--jobs", type=int, default=4, help="processus de référence en parallèle")
    parser.add_argument("--no-compare", action="store_true",
                        help="écrit cases.jsonl et expected.jsonl, sans lancer cargo test")
    options = parser.parse_args()
    if options.seed is None:
        options.seed = int(time.time())
    out = Path(options.out) if options.out else ROOT / "target/parity-random" / str(options.seed)
    out = out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    print(f"graine {options.seed}, dossier {out}", flush=True)

    gen = draw(options, options.reference_dir)
    with open(out / "cases.jsonl", "w", encoding="utf-8") as handle:
        for case in gen.cases:
            handle.write(json.dumps(case, ensure_ascii=True) + "\n")
    digest = hashlib.sha1((out / "cases.jsonl").read_bytes()).hexdigest()
    conversions = sum(len(case["input"]) for case in gen.cases)
    print(f"{len(gen.cases)} invocations, {conversions} conversions, sha1 {digest}", flush=True)
    for family in sorted(gen.counts):
        print(f"  {family}: {gen.counts[family]}")
    expected = record(Reference(options.reference_dir), gen.cases, options.jobs)
    with open(out / "expected.jsonl", "w", encoding="utf-8") as handle:
        for row in expected:
            handle.write(json.dumps(row, ensure_ascii=True) + "\n")
    meta = {"seed": options.seed, "scale": options.scale, "random": options.random,
            "points": options.points, "world": options.world,
            "named_datums": options.named_datums, "sha1": digest,
            "invocations": len(gen.cases), "conversions": conversions}
    (out / "meta.json").write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
    if options.no_compare:
        return 0

    env = {**os.environ, "PARITY_RANDOM_DIR": str(out), "PARITY_REPORT": str(out / "report.tsv")}
    command = ["cargo", "test", "--test", "parity", "parity_random_against_proj_9_9_0", "--",
               "--ignored", "--nocapture", "--test-threads=1"]
    proc = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, text=True, check=False)
    (out / "compare.log").write_text(proc.stdout + proc.stderr, encoding="utf-8")
    summary = [line for line in proc.stdout.splitlines()
               if re.match(r"\s*(parité contre|identiques|hors seuil)", line)]
    print("\n".join(summary))
    print(f"test result : {'ok' if proc.returncode == 0 else 'ÉCARTS'} ; rapport {out / 'report.tsv'}")
    return proc.returncode


if __name__ == "__main__":
    sys.exit(main())
