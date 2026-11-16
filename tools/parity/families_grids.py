"""Familles avec grilles : opérations, paires et pipelines `cct` à une étape."""

import os
import random
import sqlite3
import subprocess
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from common import ROOT, cs2cs_kinds, lon_width, overlap


GRID_METHODS = ("hgridshift", "gridshift", "vgridshift", "geoid_like", "geocentricoffset", "velocity_grid")
GRID_DIRECT = ("hgridshift", "gridshift", "vgridshift")
WORLD_EXT = (-89.0, 89.0, -180.0, 180.0)
GRID_OPS = ROOT / "tests/parity/grids/ops.tsv"
GRID_LIST = ROOT / "tests/parity/grids/grids.txt"
MAX_GRID_BYTES = 30_000_000


def grid_extents(db):
    extents = {}
    for o_auth, o_code, south, north, west, east in db.execute(
        "select u.object_auth_name, u.object_code, x.south_lat, x.north_lat, x.west_lon, x.east_lon "
        "from usage u join extent x on x.auth_name=u.extent_auth_name and x.code=u.extent_code "
        "where u.object_table_name='grid_transformation' and x.deprecated=0 and x.south_lat is not null"
    ):
        area = (north - south) * lon_width(west, east)
        key = (o_auth, str(o_code))
        if key not in extents or area < extents[key][0]:
            extents[key] = (area, (south, north, west, east))
    return {key: value[1] for key, value in extents.items()}


def grid_candidates(db_path):
    """Opérations de grille EPSG non dépréciées dont les fichiers ont une URL CDN."""
    db = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
    alt = {orig: (name, method, url) for orig, name, method, url in db.execute(
        "select original_grid_name, proj_grid_name, proj_method, url from grid_alternatives "
        "where url is not null")}
    extents = grid_extents(db)
    ops = []
    for auth, code, name, s_a, s_c, t_a, t_c, g1, g2 in db.execute(
        "select auth_name, code, name, source_crs_auth_name, source_crs_code, target_crs_auth_name, "
        "target_crs_code, grid_name, grid2_name from grid_transformation where deprecated=0 "
        "and auth_name='EPSG' order by code"
    ):
        if g1 not in alt or alt[g1][1] not in GRID_METHODS or (s_a, t_a) != ("EPSG", "EPSG"):
            continue
        if g2 is not None and g2 not in alt:
            continue
        files = [alt[g1][0]] + ([alt[g2][0]] if g2 is not None and alt[g2][0] != alt[g1][0] else [])
        urls = [alt[g1][2]] + ([alt[g2][2]] if g2 is not None else [])
        if "" in files:
            continue
        ext = extents.get((auth, str(code)))
        if ext is None:
            continue
        ops.append({"id": f"EPSG:{code}", "name": name, "method": alt[g1][1], "src": f"EPSG:{s_c}",
                    "dst": f"EPSG:{t_c}", "ext": ext, "files": files, "urls": urls})
    return ops


def head_bytes(url):
    proc = subprocess.run(["curl", "-sSIL", "--max-time", "60", url], capture_output=True, text=True, check=False)
    sizes = [int(line.split(":")[1]) for line in proc.stdout.splitlines() if line.lower().startswith("content-length:")]
    if proc.returncode != 0 or not sizes:
        raise SystemExit(f"taille inconnue : {url}")
    return sizes[-1]


def select_grid_ops(candidates, catalog, count, seed):
    """Tire `count` opérations, équilibrées par (méthode, pays) et sans fichier en double."""
    urls = {url: name for op in candidates for name, url in zip(op["files"], op["urls"])}
    with ThreadPoolExecutor(8) as pool:
        sizes = dict(zip(urls, pool.map(head_bytes, urls)))
    usable = []
    for op in candidates:
        if any(sizes[url] > MAX_GRID_BYTES for url in op["urls"]):
            continue
        if not all(_has_crs(catalog, ref) for ref in (op["src"], op["dst"])):
            continue
        usable.append(op)
    groups = defaultdict(list)
    for op in usable:
        country = op["files"][0].split("_")[0] if "_" in op["files"][0] else "other"
        groups[(op["method"], country)].append(op)
    rng = random.Random(seed)
    keys = sorted(groups)
    for key in keys:
        rng.shuffle(groups[key])
    picked, seen = [], set()
    while len(picked) < count and any(groups.values()):
        for key in keys:
            if len(picked) >= count or not groups[key]:
                continue
            op = groups[key].pop()
            if any(name in seen for name in op["files"]):
                continue
            seen.update(op["files"])
            picked.append(op)
    return picked, {name: sizes[url] for url, name in urls.items()}


def _has_crs(catalog, ref):
    return ref in catalog.by_ref or any(c["vert"] == ref for c in catalog.by_kind["compound"])


def write_grid_selection(ops, sizes, ops_path, list_path):
    with open(ops_path, "w", encoding="utf-8") as out:
        out.write("# id\tnom\tméthode\tsource\tcible\temprise S,N,W,E\tfichiers\n")
        for op in ops:
            south, north, west, east = op["ext"]
            out.write("\t".join([op["id"], op["name"], op["method"], op["src"], op["dst"],
                                 f"{south},{north},{west},{east}", ",".join(op["files"])]) + "\n")
    names = sorted({name for op in ops for name in op["files"]})
    with open(list_path, "w", encoding="utf-8") as out:
        out.write("# fichiers de tests/parity/grids/ops.tsv, téléchargés sur https://cdn.proj.org/<nom>\n")
        for name in names:
            out.write(f"{name}\t{sizes[name]}\n")
    print(f"{len(ops)} operations, {len(names)} files, {sum(sizes[n] for n in names) / 1e6:.1f} MB")


def load_grid_ops(path):
    ops = []
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        op_id, name, method, src, dst, extent, files = line.split("\t")
        south, north, west, east = (float(v) for v in extent.split(","))
        ops.append({"id": op_id, "name": name, "method": method, "src": src, "dst": dst,
                    "ext": (south, north, west, east), "files": files.split(",")})
    return ops


def chosen_proj_string(lines):
    """Chaîne PROJ de la première opération de `projinfo` dont toutes les grilles existent."""
    for section in "\n".join(lines).split("Operation No.")[1:]:
        head, marker, tail = section.partition("PROJ string:")
        if not marker or "missing" in head or "not found on the system" in section:
            continue
        block = []
        for line in tail.splitlines():
            if line.strip():
                block.append(line.strip())
            elif block:
                break
        return " ".join(block)
    return ""


def grid_names(proj_string):
    names = []
    for token in proj_string.split():
        if token.startswith(("+grids=", "+nadgrids=")):
            names += [part.lstrip("@") for part in token.split("=", 1)[1].split(",") if part]
    return names


def crs_members(gen, ref, ext):
    """CRS d'une opération de grille : le CRS lui-même, ou les composés qui ont ce vertical."""
    if ref in gen.cat.by_ref and gen.cat.by_ref[ref]["kind"] != "vert":
        return [gen.cat.by_ref[ref]]
    return [c for c in gen.cat.by_kind["compound"] if c["vert"] == ref and overlap(c["ext"], ext)]


def grid_pairs(gen, op, grids_dir, cache):
    """Couples (source, cible) dont la première opération PROJ utilise une grille de `grids_dir`."""
    sources, targets = crs_members(gen, op["src"], op["ext"]), crs_members(gen, op["dst"], op["ext"])
    if not sources or not targets:
        return []
    pairs = []
    for _ in range(4):
        a, b = gen.rng.choice(sources), gen.rng.choice(targets)
        if a is b or (a["ref"], b["ref"]) in cache:
            continue
        _, lines = gen.ref.run("projinfo", ["-s", a["ref"], "-t", b["ref"], "-o", "PROJ"], [])
        names = grid_names(chosen_proj_string(lines))
        ok = bool(names) and all(os.path.exists(os.path.join(grids_dir, n)) for n in names)
        cache[(a["ref"], b["ref"])] = ok
        if ok:
            pairs.append((a, b))
    return pairs


def fam_grids(gen, ops, grids_dir, points):
    cache = {}
    for op in ops:
        method = op["method"]
        for src, dst in grid_pairs(gen, op, grids_dir, cache):
            args = ["-d", "12", src["ref"], "+to", dst["ref"]]
            gen.emit(f"cs2cs-grid-in-{method}", "cs2cs", args,
                     gen.sample(src, points, ext=op["ext"]), cs2cs_kinds(dst))
            gen.emit(f"cs2cs-grid-out-{method}", "cs2cs", args,
                     gen.sample(src, max(4, points // 2), ext=WORLD_EXT), cs2cs_kinds(dst))
            gen.emit(f"cs2cs-grid-inv-{method}", "cs2cs", ["-d", "12", "-I", src["ref"], "+to", dst["ref"]],
                     gen.sample(dst, points, ext=op["ext"]), cs2cs_kinds(src))


def fam_grid_cct(gen, ops, points):
    """Pipelines directs `cct` autour d'une seule étape de grille, sans CRS."""
    for op in ops:
        if op["method"] not in GRID_DIRECT:
            continue
        step = op["method"]
        south, north, west, east = op["ext"]
        lines = [f"{gen.rng.uniform(west, west + lon_width(west, east)):.9f} "
                 f"{gen.rng.uniform(south, north):.9f} {gen.rng.uniform(-100, 4000):.3f} "
                 f"{gen.rng.uniform(2000, 2020):.1f}" for _ in range(points)]
        template = (f"+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad "
                    f"+step +proj={step} +grids={op['files'][0]} "
                    f"+step +proj=unitconvert +xy_in=rad +xy_out=deg")
        gen.emit(f"cct-grid-{op['method']}", "cct", ["-d", "12", *template.split()], lines,
                 ["deg", "deg", "m", "t"])


def build_grids(gen, grids_dir, ops_path, points):
    ops = load_grid_ops(ops_path)
    present = [op for op in ops if all(os.path.exists(os.path.join(grids_dir, f)) for f in op["files"])]
    print(f"{len(present)} of {len(ops)} operations have their grid files in {grids_dir}")
    fam_grids(gen, present, grids_dir, points)
    fam_grid_cct(gen, present, max(5, points // 2))
