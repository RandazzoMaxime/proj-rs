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
import sqlite3
import subprocess
import sys
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
METRE, DEGREE = 9001, 9122
GEOG_KINDS = {
    "geographic 2D": ("geog2d", [DEGREE, DEGREE]),
    "geographic 3D": ("geog3d", [DEGREE, DEGREE, METRE]),
    "geocentric": ("geocent", [METRE, METRE, METRE]),
}
KIND_OF = {
    "geog2d": ["deg", "deg"],
    "geog3d": ["deg", "deg", "m"],
    "geocent": ["m", "m", "m"],
    "proj": ["m", "m"],
}
ERROR_TOKENS = {"*", "inf", "-inf", "nan"}


def wrap(lon):
    return (lon + 180.0) % 360.0 - 180.0


def lon_width(west, east):
    width = (east - west) % 360.0
    return width if width > 0 else 360.0


def overlap(e1, e2):
    s1, n1, w1, x1 = e1
    s2, n2, w2, x2 = e2
    if max(s1, s2) > min(n1, n2):
        return False
    width1, width2 = lon_width(w1, x1), lon_width(w2, x2)
    for shift in (-360.0, 0.0, 360.0):
        if max(w1, w2 + shift) < min(w1 + width1, w2 + shift + width2):
            return True
    return False


def valid_line(line, dims):
    tokens = line.split()
    return len(tokens) == dims and not (set(tokens) & ERROR_TOKENS)


def crs_kinds(crs):
    if crs["kind"] == "compound":
        return ["deg", "deg", "m"] if crs["horiz"] == "geog2d" else ["m", "m", "m"]
    return list(KIND_OF[crs["kind"]])


def cs2cs_kinds(crs, time=False):
    """cs2cs écrit toujours trois valeurs (z = 0 si absent), quatre avec le temps."""
    kinds = crs_kinds(crs)
    if len(kinds) == 2:
        kinds.append("m")
    return kinds + (["t"] if time else [])


def datum_label(a, b):
    return "same-datum" if a["datum"] == b["datum"] else "datum-diff"


class Catalog:
    """CRS EPSG non dépréciés, en degrés ou en mètres, avec leur emprise (usage)."""

    def __init__(self, db_path):
        db = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
        self.axes = defaultdict(list)
        for cs_auth, cs_code, abbrev, uom in db.execute(
            "select coordinate_system_auth_name, coordinate_system_code, abbrev, uom_code "
            "from axis where uom_auth_name='EPSG' "
            "order by coordinate_system_auth_name, coordinate_system_code, coordinate_system_order"
        ):
            self.axes[(cs_auth, str(cs_code))].append((abbrev, int(uom)))
        extents = {}
        for table, o_auth, o_code, south, north, west, east in db.execute(
            "select u.object_table_name, u.object_auth_name, u.object_code, x.south_lat, "
            "x.north_lat, x.west_lon, x.east_lon from usage u join extent x "
            "on x.auth_name=u.extent_auth_name and x.code=u.extent_code "
            "where x.deprecated=0 and x.south_lat is not null order by u.rowid"
        ):
            area = (north - south) * lon_width(west, east)
            key = (table, o_auth, str(o_code))
            if key not in extents or area < extents[key][0]:
                extents[key] = (area, (south, north, west, east))
        self.extents = {key: value[1] for key, value in extents.items()}
        self.by_kind = defaultdict(list)
        self.by_ref = {}
        self._load_geodetic(db)
        self._load_projected(db)
        self._load_vertical(db)
        self._load_compound(db)

    def _uoms(self, cs_auth, cs_code):
        return [uom for _, uom in self.axes.get((cs_auth, str(cs_code)), [])]

    def _add(self, crs):
        self.by_kind[crs["kind"]].append(crs)
        self.by_ref[crs["ref"]] = crs

    def _load_geodetic(self, db):
        for auth, code, typ, cs_a, cs_c, d_a, d_c in db.execute(
            "select auth_name, code, type, coordinate_system_auth_name, coordinate_system_code, "
            "datum_auth_name, datum_code from geodetic_crs "
            "where deprecated=0 and auth_name='EPSG' order by code"
        ):
            if typ not in GEOG_KINDS:
                continue
            kind, uoms = GEOG_KINDS[typ]
            ax = self.axes.get((cs_a, str(cs_c)), [])
            if [uom for _, uom in ax] != uoms:
                continue
            self._add({
                "kind": kind, "ref": f"EPSG:{code}", "ext": self.extents.get(("geodetic_crs", auth, str(code))),
                "datum": (d_a, str(d_c)), "dims": len(uoms), "lat_first": ax[0][0] in ("Lat", "N"),
            })

    def _load_projected(self, db):
        for auth, code, cs_a, cs_c, b_c in db.execute(
            "select auth_name, code, coordinate_system_auth_name, coordinate_system_code, "
            "geodetic_crs_code from projected_crs where deprecated=0 and auth_name='EPSG' order by code"
        ):
            base = self.by_ref.get(f"EPSG:{b_c}")
            ext = self.extents.get(("projected_crs", auth, str(code)))
            if base is None or ext is None or self._uoms(cs_a, cs_c) != [METRE, METRE]:
                continue
            self._add({"kind": "proj", "ref": f"EPSG:{code}", "ext": ext, "datum": base["datum"],
                       "dims": 2, "lat_first": False})

    def _load_vertical(self, db):
        for auth, code, cs_a, cs_c in db.execute(
            "select auth_name, code, coordinate_system_auth_name, coordinate_system_code "
            "from vertical_crs where deprecated=0 and auth_name='EPSG' order by code"
        ):
            if self._uoms(cs_a, cs_c) == [METRE]:
                self._add({"kind": "vert", "ref": f"EPSG:{code}",
                           "ext": self.extents.get(("vertical_crs", auth, str(code))),
                           "datum": None, "dims": 1, "lat_first": False})

    def _load_compound(self, db):
        for auth, code, h_c, v_c in db.execute(
            "select auth_name, code, horiz_crs_code, vertical_crs_code from compound_crs "
            "where deprecated=0 and auth_name='EPSG' order by code"
        ):
            horiz = self.by_ref.get(f"EPSG:{h_c}")
            vert = self.by_ref.get(f"EPSG:{v_c}")
            ext = self.extents.get(("compound_crs", auth, str(code)))
            if horiz is None or vert is None or vert["kind"] != "vert" or ext is None:
                continue
            if horiz["kind"] not in ("geog2d", "proj"):
                continue
            self._add({"kind": "compound", "ref": f"EPSG:{code}", "ext": ext, "datum": horiz["datum"],
                       "dims": 3, "lat_first": horiz["lat_first"], "horiz": horiz["kind"],
                       "vert": f"EPSG:{v_c}"})


class Reference:
    """Lance les binaires de la référence avec l'environnement de la suite."""

    def __init__(self, ref_dir, grids_dir=None):
        self.ref = ref_dir
        self.env = os.environ.copy()
        search = f"{ref_dir}/build/data:{grids_dir}" if grids_dir \
            else f"{ref_dir}/build/data:{ROOT}/data:{ROOT}/data/tests"
        self.env.update({
            "PROJ_DATA": search,
            "PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY": "YES",
            "PROJ_NETWORK": "OFF",
            "LD_LIBRARY_PATH": f"{ref_dir}/build/lib",
        })

    def run(self, tool, args, lines):
        text = "".join(f"{line}\n" for line in lines)
        proc = subprocess.run([f"{self.ref}/build/bin/{tool}", *args], input=text,
                              capture_output=True, text=True, env=self.env, check=False)
        return proc.returncode, proc.stdout.splitlines()

    def grid_free(self, src, dst):
        proc = subprocess.run([f"{self.ref}/build/bin/projinfo", "-s", src, "-t", dst],
                              capture_output=True, text=True, env=self.env, check=False)
        return proc.returncode == 0 and "grid missing" not in proc.stdout


def projection_defs(ids):
    """Définitions de `bench_definition` (src/ported.rs) ; sinon `+proj=ID +ellps=WGS84`."""
    ported = (ROOT / "src/ported.rs").read_text(encoding="utf-8")
    start = ported.index("pub fn bench_definition")
    body = ported[start:ported.index("\n}\n", start)]
    table = {}
    pattern = re.compile(r'((?:"\w+"\s*\|\s*)*"\w+")\s*=>\s*(?:\{\s*)?(?:format!\(\s*)?"([^"]*)"')
    for names_raw, template in pattern.findall(body):
        if any(s in template for s in ("grids=", "model=", "file=", "tests/", "urn:", "pipeline")):
            continue
        for name in re.findall(r'"(\w+)"', names_raw):
            table.setdefault(name, template.replace("{name}", name))
    return {pid: table.get(pid, f"+proj={pid} +ellps=WGS84") for pid in ids}


class Generator:
    def __init__(self, catalog, reference, rng, points=6, world=0.0):
        self.points = points
        self.world = world
        self.cat = catalog
        self.ref = reference
        self.rng = rng
        self.pools = {}
        self.grids = {}
        self.cases = []
        self.counts = defaultdict(int)
        self.dropped = defaultdict(int)
        self.ids = []

    def geo_points(self, ext, count):
        south, north, west, east = ext
        width = lon_width(west, east)
        eps = 1e-3
        fixed = [(south + eps, west + eps), (south + eps, west + width - eps),
                 (north - eps, west + eps), (north - eps, west + width - eps)]
        if south <= -89.5:
            fixed.append((-89.9, west + width / 2))
        if north >= 89.5:
            fixed.append((89.9, west + width / 2))
        if width >= 359.0:
            fixed += [(south + 10, 179.9), (south + 10, -179.9)]
        points = [(lat, wrap(lon)) for lat, lon in fixed]
        outside = int(count * self.world)
        while len(points) < count - outside:
            lat = self.rng.uniform(south, north)
            points.append((lat, wrap(west + width * self.rng.random())))
        while len(points) < count:
            points.append((self.rng.uniform(-89.0, 89.0), self.rng.uniform(-179.9, 179.9)))
        return points

    def pool(self, crs, ext=None):
        """Coordonnées d'entrée valides dans le CRS, en ordre d'axes du CRS (mis en cache).

        `ext` (south, north, west, east) remplace l'emprise du CRS : emprise d'une grille.
        """
        ext = crs["ext"] if ext is None else ext
        if (crs["ref"], ext) in self.pools:
            return self.pools[(crs["ref"], ext)]
        lines = []
        if crs["kind"] in ("geog2d", "geog3d"):
            for lat, lon in self.geo_points(ext, max(40, 4 * self.points)):
                first, second = (lat, lon) if crs["lat_first"] else (lon, lat)
                parts = [f"{first:.9f}", f"{second:.9f}"]
                if crs["dims"] == 3:
                    parts.append(f"{self.rng.uniform(-100, 4000):.3f}")
                lines.append(" ".join(parts))
        else:
            src, dims = ("EPSG:4326", 2) if crs["kind"] == "proj" else ("EPSG:4979", 3)
            inputs = []
            for lat, lon in self.geo_points(ext, max(48, 5 * self.points)):
                tail = "" if dims == 2 else f" {self.rng.uniform(-100, 4000):.3f}"
                inputs.append(f"{lat:.9f} {lon:.9f}{tail}")
            _, out = self.ref.run("cs2cs", ["-d", "12", src, "+to", crs["ref"]], inputs)
            if len(out) == len(inputs):
                lines = [" ".join(o.split()[:dims]) for o in out if valid_line(o, 3)]
        self.pools[(crs["ref"], ext)] = lines
        return lines

    def sample(self, crs, k=None, ext=None):
        pool = self.pool(crs, ext)
        if len(pool) < 3:
            return []
        return self.rng.sample(pool, min(k or self.points, len(pool)))

    def pick(self, group_a, group_b, overlap_pref=0.8, same_datum=False):
        """Une paire (a, b) dont les emprises se recouvrent (souvent), sans grille manquante."""
        for _ in range(200):
            a = self.rng.choice(group_a)
            candidates = [c for c in group_b if c is not a and c["datum"] == a["datum"]] \
                if same_datum else group_b
            if not candidates:
                continue
            b = self.rng.choice(candidates)
            if a is b or a["ext"] is None or b["ext"] is None:
                continue
            if self.rng.random() < overlap_pref and not overlap(a["ext"], b["ext"]):
                continue
            key = (a["ref"], b["ref"])
            if key not in self.grids:
                self.grids[key] = self.ref.grid_free(a["ref"], b["ref"])
            if not self.grids[key]:
                self.dropped["grid"] += 1
                continue
            return a, b
        return None

    def emit(self, family, tool, args, lines, kinds):
        """Ajoute l'invocation si la référence en produit au moins un résultat de la bonne forme."""
        if not lines:
            self.dropped[family] += 1
            return False
        _, out = self.ref.run(tool, args, lines)
        good = [o for o in out if not set(o.split()) & ERROR_TOKENS]
        if len(out) != len(lines) or not good or any(len(o.split()) != len(kinds) for o in good):
            self.dropped[family] += 1
            return False
        self.counts[family] += 1
        self.cases.append({
            "id": f"{family}-{self.counts[family]:04d}", "family": family, "tool": tool,
            "args": args, "input": lines, "kinds": kinds,
        })
        return True


def emit_text(gen, family, tool, args):
    """Une invocation sans entrée, comparée au texte (liste des candidats de `projinfo`)."""
    code, out = gen.ref.run(tool, args, [])
    if code != 0 or not out:
        gen.dropped[family] += 1
        return False
    gen.counts[family] += 1
    gen.cases.append({
        "id": f"{family}-{gen.counts[family]:04d}", "family": family, "tool": tool,
        "args": args, "input": [], "kinds": ["text"],
    })
    return True


def fam_projinfo(gen, want):
    """`projinfo -s A -t B --summary` : la liste des candidats, comparée au texte."""
    kinds = ["geog2d", "geog3d", "geocent", "proj", "compound"]
    counter = [0]

    def make():
        counter[0] += 1
        src = gen.cat.by_kind[kinds[counter[0] % len(kinds)]]
        dst = gen.cat.by_kind[gen.rng.choice(kinds)]
        pair = gen.pick(src, dst)
        if pair is None:
            return False
        a, b = pair
        return emit_text(gen, "projinfo-candidates", "projinfo",
                         ["-s", a["ref"], "-t", b["ref"], "--spatial-test", "intersects", "--summary"])
    return attempts(want, make)


def attempts(want, make):
    """Appelle `make()` jusqu'à `want` invocations retenues, avec une borne d'essais."""
    made = 0
    for _ in range(want * 25):
        if made >= want:
            break
        if make():
            made += 1
    return made


def fam_geo2d(gen, want):
    group = gen.cat.by_kind["geog2d"]

    counter = [0]

    def make():
        counter[0] += 1
        pair = gen.pick(group, group, same_datum=counter[0] % 3 == 0)
        if pair is None:
            return False
        a, b = pair
        return gen.emit(f"cs2cs-geo2d-{datum_label(a, b)}", "cs2cs",
                        ["-d", "12", a["ref"], "+to", b["ref"]], gen.sample(a), cs2cs_kinds(b))
    return attempts(want, make)


def fam_heights(gen, want):
    g3, geocent, proj = gen.cat.by_kind["geog3d"], gen.cat.by_kind["geocent"], gen.cat.by_kind["proj"]
    routes = [("cs2cs-h3d-geog3d-geocent", g3, geocent), ("cs2cs-h3d-geocent-proj", geocent, proj),
              ("cs2cs-h3d-geog3d-proj", g3, proj)]
    counter = [0]

    def make():
        family, src, dst = routes[counter[0] % len(routes)]
        counter[0] += 1
        pair = gen.pick(src, dst)
        if pair is None:
            return False
        a, b = pair
        return gen.emit(family, "cs2cs", ["-d", "12", a["ref"], "+to", b["ref"]],
                        gen.sample(a), cs2cs_kinds(b))
    return attempts(want, make)


def fam_proj(gen, want):
    group = gen.cat.by_kind["proj"]
    counter = [0]

    def make():
        counter[0] += 1
        pair = gen.pick(group, group, same_datum=counter[0] % 2 == 0)
        if pair is None:
            return False
        a, b = pair
        return gen.emit(f"cs2cs-proj-{datum_label(a, b)}", "cs2cs",
                        ["-d", "12", a["ref"], "+to", b["ref"]], gen.sample(a), cs2cs_kinds(b))
    return attempts(want, make)


def fam_compound(gen, want):
    comp, g3 = gen.cat.by_kind["compound"], gen.cat.by_kind["geog3d"]
    counter = [0]

    def make():
        counter[0] += 1
        if counter[0] % 2:
            pair, family = gen.pick(comp, g3), "cs2cs-compound-geog3d"
        else:
            pair, family = gen.pick(g3, comp), "cs2cs-geog3d-compound"
        if pair is None:
            return False
        a, b = pair
        return gen.emit(family, "cs2cs", ["-d", "12", a["ref"], "+to", b["ref"]],
                        gen.sample(a), cs2cs_kinds(b))
    return attempts(want, make)


def fam_vertical(gen, want):
    comp = gen.cat.by_kind["compound"]

    def make():
        pair = gen.pick(comp, comp)
        if pair is None:
            return False
        a, b = pair
        return gen.emit("cs2cs-vertical-compound", "cs2cs",
                        ["-d", "12", a["ref"], "+to", b["ref"]], gen.sample(a), cs2cs_kinds(b))
    return attempts(want, make)


def fam_inverse(gen, want):
    proj = gen.cat.by_kind["proj"]

    def make():
        pair = gen.pick(proj, proj)
        if pair is None:
            return False
        a, b = pair
        return gen.emit("cs2cs-inverse", "cs2cs", ["-d", "12", "-I", a["ref"], "+to", b["ref"]],
                        gen.sample(b), cs2cs_kinds(a))
    return attempts(want, make)


def fam_options(gen, want):
    g2, g3, proj = gen.cat.by_kind["geog2d"], gen.cat.by_kind["geog3d"], gen.cat.by_kind["proj"]
    third = max(1, want // 3)

    def option_case(option, group_a, group_b, label):
        def make():
            pair = gen.pick(group_a, group_b)
            if pair is None:
                return False
            a, b = pair
            kinds = cs2cs_kinds(b)
            return gen.emit(label, "cs2cs", ["-d", "12", *option, a["ref"], "+to", b["ref"]],
                            gen.sample(a), kinds)
        return make

    total = attempts(third, option_case(["--3d"], g3, proj, "cs2cs-option-3d"))
    total += attempts(third, option_case(["--only-best"], proj, proj, "cs2cs-option-only-best"))
    total += attempts(third, option_case(["--no-ballpark"], g2, g2, "cs2cs-option-no-ballpark"))
    return total


def fam_epoch(gen, want):
    itrf = gen.cat.by_ref.get("EPSG:7789")
    etrs = gen.cat.by_ref.get("EPSG:4936")
    wgs3 = gen.cat.by_ref.get("EPSG:4979")
    each = max(1, want // 4)

    def epoch():
        return f"{gen.rng.uniform(2000, 2020):.1f}"

    def source_epoch():
        return gen.emit("cs2cs-epoch-source", "cs2cs",
                        ["-d", "12", "--s_epoch", epoch(), itrf["ref"], "+to", etrs["ref"]],
                        gen.sample(itrf), cs2cs_kinds(etrs))

    def target_epoch():
        return gen.emit("cs2cs-epoch-target", "cs2cs",
                        ["-d", "12", "--t_epoch", epoch(), etrs["ref"], "+to", itrf["ref"]],
                        gen.sample(etrs), cs2cs_kinds(itrf))

    def fourth_coordinate():
        lines = [f"{line} {epoch()}" for line in gen.sample(itrf)]
        return gen.emit("cs2cs-epoch-time", "cs2cs",
                        ["-d", "12", itrf["ref"], "+to", etrs["ref"]], lines, cs2cs_kinds(etrs, True))

    def fourth_from_wgs():
        lines = [f"{line} {epoch()}" for line in gen.sample(wgs3)]
        return gen.emit("cs2cs-epoch-time", "cs2cs",
                        ["-d", "12", wgs3["ref"], "+to", itrf["ref"]], lines, cs2cs_kinds(itrf, True))

    return sum(attempts(each, make) for make in
               (source_epoch, target_epoch, fourth_coordinate, fourth_from_wgs))


CCT_TEMPLATES = [
    "+proj=pipeline +step +proj=cart +ellps=GRS80 +step +proj=helmert +x=12 +y=-34 +z=56 "
    "+s=1.5 +step +inv +proj=cart +ellps=WGS84",
    "+proj=pipeline +step +proj=axisswap +order=2,1 +step +proj=axisswap +order=2,1",
    "+proj=pipeline +step +proj=merc +ellps=WGS84 +step +inv +proj=merc +ellps=WGS84",
    "+proj=pipeline +step +proj=utm +zone=31 +ellps=WGS84 +step +inv +proj=utm +zone=31 +ellps=WGS84",
    "+proj=pipeline +step +proj=unitconvert +xy_in=deg +xy_out=rad "
    "+step +proj=unitconvert +xy_in=rad +xy_out=deg",
    "+proj=pipeline +step +proj=unitconvert +z_in=m +z_out=km +step +proj=unitconvert +z_in=km +z_out=m",
    "+proj=pipeline +step +proj=cart +ellps=WGS84 +step +proj=helmert +x=-87 +y=-96 +z=-120 "
    "+step +inv +proj=cart +ellps=intl",
    "+proj=pipeline +step +proj=webmerc +ellps=WGS84 +step +inv +proj=webmerc +ellps=WGS84",
    "+proj=pipeline +step +proj=geoc +ellps=WGS84 +step +inv +proj=geoc +ellps=WGS84",
    "+proj=pipeline +step +proj=lcc +lat_0=46.5 +lat_1=49 +lat_2=44 +lon_0=3 +x_0=700000 "
    "+y_0=6600000 +ellps=GRS80 +step +inv +proj=lcc +lat_0=46.5 +lat_1=49 +lat_2=44 +lon_0=3 "
    "+x_0=700000 +y_0=6600000 +ellps=GRS80",
    "+proj=pipeline +step +proj=tmerc +lon_0=3 +k=0.9996 +ellps=GRS80 "
    "+step +inv +proj=tmerc +lon_0=3 +k=0.9996 +ellps=GRS80",
    "+proj=pipeline +step +proj=unitconvert +xy_in=m +xy_out=km "
    "+step +proj=unitconvert +xy_in=km +xy_out=m",
]


def fam_cct(gen, per_template=2, points=5):
    made = 0
    for template in CCT_TEMPLATES:
        for _ in range(per_template):
            lines = [f"{gen.rng.uniform(-2, 8):.9f} {gen.rng.uniform(40, 56):.9f} "
                     f"{gen.rng.uniform(-100, 4000):.3f} {gen.rng.uniform(2000, 2020):.1f}"
                     for _ in range(points)]
            made += gen.emit("cct-pipeline", "cct", ["-d", "12", *template.split()], lines,
                             ["deg", "deg", "m", "t"])
    return made


def local_points(gen, count):
    return [f"{gen.rng.uniform(-30, 30):.9f} {gen.rng.uniform(-50, 50):.9f}" for _ in range(count)]


def fam_projections(gen, inv_count=50, fac_count=25, per=3):
    defs = projection_defs(gen.ids)
    for pid in gen.ids:
        lines = local_points(gen, per) + [f"{gen.rng.uniform(-170, 170):.9f} "
                                          f"{gen.rng.uniform(-80, 80):.9f}" for _ in range(per)]
        gen.emit("proj-fwd", "proj", ["-f", "%.12f", *defs[pid].split()], lines, ["m", "m"])
    inverse = 0
    for pid in gen.ids:
        if inverse >= inv_count:
            break
        _, forward = gen.ref.run("proj", ["-f", "%.12f", *defs[pid].split()],
                                 local_points(gen, 2 * per))
        good = [o for o in forward if valid_line(o, 2)]
        if len(good) >= 3 and gen.emit("proj-inv", "proj", ["-I", "-f", "%.12f", *defs[pid].split()],
                                       good[:2 * per], ["deg", "deg"]):
            inverse += 1
    step = max(1, len(gen.ids) // fac_count)
    for pid in gen.ids[::step][:fac_count]:
        gen.emit("proj-factors", "proj", ["-S", "-f", "%.12f", *defs[pid].split()],
                 local_points(gen, 2 * per),
                 ["m", "m"] + ["fac"] * 6)


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


def grid_pairs(gen, op, grids_dir, cache):
    """Couples (source, cible) dont la première opération PROJ utilise une grille de `grids_dir`."""
    def members(ref):
        if ref in gen.cat.by_ref and gen.cat.by_ref[ref]["kind"] != "vert":
            return [gen.cat.by_ref[ref]]
        return [c for c in gen.cat.by_kind["compound"] if c["vert"] == ref and overlap(c["ext"], op["ext"])]

    sources, targets = members(op["src"]), members(op["dst"])
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
    options = parser.parse_args()
    catalog = Catalog(Path(options.reference_dir) / "build/data/proj.db")
    if options.grid_select:
        candidates = grid_candidates(Path(options.reference_dir) / "build/data/proj.db")
        ops, sizes = select_grid_ops(candidates, catalog, options.grid_count, options.seed)
        write_grid_selection(ops, sizes, Path(options.grid_ops), GRID_LIST)
        return
    gen = Generator(catalog, Reference(options.reference_dir, options.grids_dir),
                    random.Random(options.seed), points=options.points, world=options.world)
    if options.grids_dir:
        build_grids(gen, options.grids_dir, options.grid_ops, options.points)
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
