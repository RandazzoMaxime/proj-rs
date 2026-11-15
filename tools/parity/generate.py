#!/usr/bin/env python3
"""Corpus de parité contre PROJ 9.9.0 : écrit tests/parity/cases.jsonl.

Stdlib seulement. Les CRS et leurs emprises viennent de la proj.db de la
référence. La référence sert ici à calculer les coordonnées d'entrée hors
WGS 84, à écarter les paires qui exigent une grille absente, et à écarter les
invocations qui ne produisent aucun résultat. Les résultats attendus ne sont
pas écrits ici : c'est record.py. Graine fixe, sortie déterministe.
"""

import argparse
import json
import os
import random
import re
import sys
from pathlib import Path

from catalog import Catalog
from common import ROOT
from families_cs2cs import (
    fam_compound,
    fam_epoch,
    fam_geo2d,
    fam_heights,
    fam_inverse,
    fam_options,
    fam_proj,
    fam_projinfo,
    fam_vertical,
)
from families_grids import (
    GRID_LIST,
    GRID_OPS,
    build_grids,
    grid_candidates,
    select_grid_ops,
    write_grid_selection,
)
from families_export import fam_projinfo_export
from families_geod import fam_geod
from families_proj import fam_cct, fam_projections
from generator import Generator
from reference import Reference


def build(gen, scale=1, projinfo=0):
    gen.ids = re.findall(r'id: "([^"]+)"', (ROOT / "src/listing.rs").read_text(encoding="utf-8"))
    fam_geo2d(gen, 90 * scale)
    fam_heights(gen, 90 * scale)
    fam_proj(gen, 100 * scale)
    fam_compound(gen, 40 * scale)
    fam_vertical(gen, 15 * scale)
    fam_inverse(gen, 30 * scale)
    fam_options(gen, 45 * scale)
    fam_epoch(gen, 40 * scale)
    fam_cct(gen, per_template=2 * scale)
    fam_projections(gen, inv_count=50 * scale, fac_count=25 * scale, per=3 * scale)
    if projinfo:
        fam_projinfo(gen, projinfo)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--seed", type=int, default=20261115)
    parser.add_argument("--reference-dir", default=os.environ.get(
        "PROJ_REFERENCE_DIR", str(ROOT / "target/proj-reference")))
    parser.add_argument("--out", default=str(ROOT / "tests/parity/cases.jsonl"))
    parser.add_argument("--scale", type=int, default=1, help="multiplie le nombre d'invocations")
    parser.add_argument("--points", type=int, default=6, help="points par invocation cs2cs")
    parser.add_argument("--world", type=float, default=0.0,
                        help="part des points tirés dans le monde entier, hors emprise comprise")
    parser.add_argument("--projinfo", type=int, default=0, help="paires projinfo --summary")
    parser.add_argument("--grid-select", action="store_true",
                        help="tire les opérations de grille (réseau : tailles CDN) : écrit ops.tsv et grids.txt")
    parser.add_argument("--grid-count", type=int, default=150, help="opérations tirées par --grid-select")
    parser.add_argument("--grids-dir", help="corpus avec grilles : seules les paires dont PROJ utilise "
                        "une grille présente dans ce dossier")
    parser.add_argument("--grid-ops", default=str(GRID_OPS), help="liste d'opérations (ops.tsv)")
    parser.add_argument("--geod", type=int, default=0,
                        help="familles geod seules (N par famille), pour tests/parity/geod")
    parser.add_argument("--export", type=int, default=0, metavar="N",
                        help="corpus projinfo-export seul : environ N objets (tests/parity/export)")
    options = parser.parse_args()
    catalog = Catalog(Path(options.reference_dir) / "build/data/proj.db")
    if options.grid_select:
        candidates = grid_candidates(Path(options.reference_dir) / "build/data/proj.db")
        ops, sizes = select_grid_ops(candidates, catalog, options.grid_count, options.seed)
        write_grid_selection(ops, sizes, Path(options.grid_ops), GRID_LIST)
        return
    gen = Generator(catalog, Reference(options.reference_dir, options.grids_dir),
                    random.Random(options.seed), points=options.points, world=options.world)
    if options.export:
        fam_projinfo_export(gen, options.export)
    elif options.grids_dir:
        build_grids(gen, options.grids_dir, options.grid_ops, options.points)
    elif options.geod:
        fam_geod(gen, options.geod)
    else:
        build(gen, scale=options.scale, projinfo=options.projinfo)
    with open(options.out, "w", encoding="utf-8") as out:
        for case in gen.cases:
            out.write(json.dumps(case, ensure_ascii=True) + "\n")
    total = sum(len(case["input"]) for case in gen.cases)
    print(f"{len(gen.cases)} invocations, {total} conversions -> {options.out}")
    for family in sorted(gen.counts):
        print(f"  {family}: {gen.counts[family]}")
    for family in sorted(gen.dropped):
        print(f"  dropped {family}: {gen.dropped[family]}", file=sys.stderr)


if __name__ == "__main__":
    main()
