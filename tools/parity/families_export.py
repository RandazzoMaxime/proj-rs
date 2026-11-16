"""Famille `projinfo-export` : exports d'objets comparés au texte.

Objets tirés dans la proj.db de la référence : CRS de tous types, opérations,
datums et ellipsoïdes. Les projections et les conversions sont stratifiées par
méthode, pour que chaque méthode soit représentée. Chaque objet passe par
`projinfo CODE -o FORMAT -q` pour chaque format de FORMATS ; un objet sur cinq
reçoit en plus `--single-line`. Une invocation dont la référence sort en erreur
est écartée ; une sortie vide (rc 0) est gardée, elle dit aussi quelque chose.

Appelé par generate.py avec `--export N`. Stdlib seulement.
"""

import sqlite3
from collections import defaultdict

FAMILY = "projinfo-export"
FORMATS = ["PROJ", "WKT2:2019", "WKT2:2015", "WKT1_GDAL", "WKT1_ESRI", "PROJJSON"]
SINGLE_LINE_EVERY = 5
REFERENCE_COUNT = 1500
# Quotas pour REFERENCE_COUNT objets ; un autre N les met au prorata.
QUOTA = {
    "geog2d": 150, "geog3d": 60, "geocent": 25, "proj": 520, "vert": 60, "compound": 110,
    "engineering": 16, "bound": 30, "bound_string": 10,
    "helmert": 110, "grid": 80, "other": 50, "concat": 40, "conversion": 80,
    "datum": 70, "ellipsoid": 50,
}


def _quotas(count):
    if count == REFERENCE_COUNT:
        return dict(QUOTA)
    scale = count / REFERENCE_COUNT
    return {name: max(1, round(quota * scale)) for name, quota in QUOTA.items()}


def _codes(db, sql):
    return [str(row[0]) for row in db.execute(sql)]


def _grouped(db, sql):
    groups = defaultdict(list)
    for code, method in db.execute(sql):
        groups[method].append(str(code))
    return groups


def _plain(rng, codes, quota):
    codes = sorted(codes)
    rng.shuffle(codes)
    return codes[:quota]


def _stratified(rng, groups, quota):
    """Au plus `quota` codes : chaque méthode en fournit d'abord une part égale."""
    keys = sorted(groups)
    per = max(1, quota // len(keys))
    chosen, rest = [], []
    for key in keys:
        items = sorted(groups[key])
        rng.shuffle(items)
        chosen += items[:per]
        rest += items[per:]
    rng.shuffle(rest)
    return (chosen + rest)[:quota]


def _select(db, rng, quotas):
    """Codes par genre, dans l'ordre de sortie : {genre: [code, ...]}."""
    geo = defaultdict(list)
    for code, typ in db.execute(
        "select code, type from geodetic_crs where deprecated=0 and auth_name='EPSG'"
    ):
        geo[typ].append(str(code))
    picked = {
        "geog2d": _plain(rng, geo["geographic 2D"], quotas["geog2d"]),
        "geog3d": _plain(rng, geo["geographic 3D"], quotas["geog3d"]),
        "geocent": _plain(rng, geo["geocentric"], quotas["geocent"]),
        "proj": _stratified(rng, _grouped(db,
            "select p.code, c.method_code from projected_crs p join conversion c "
            "on c.auth_name=p.conversion_auth_name and c.code=p.conversion_code "
            "where p.deprecated=0 and p.auth_name='EPSG'"), quotas["proj"]),
        "vert": _plain(rng, _codes(db,
            "select code from vertical_crs where deprecated=0 and auth_name='EPSG'"), quotas["vert"]),
        "compound": _plain(rng, _codes(db,
            "select code from compound_crs where deprecated=0 and auth_name='EPSG'"), quotas["compound"]),
        "engineering": _codes(db,
            "select code from engineering_crs where deprecated=0 and auth_name='EPSG'")[:quotas["engineering"]],
    }
    used = set(picked["geog2d"])
    picked["bound"] = _plain(rng, [c for c in geo["geographic 2D"] if c not in used], quotas["bound"])
    for name, table in (("helmert", "helmert_transformation"), ("grid", "grid_transformation"),
                        ("other", "other_transformation")):
        picked[name] = _stratified(rng, _grouped(db,
            f"select code, method_code from {table} where deprecated=0 and auth_name='EPSG'"),
            quotas[name])
    picked["concat"] = _plain(rng, _codes(db,
        "select code from concatenated_operation where deprecated=0 and auth_name='EPSG'"), quotas["concat"])
    picked["conversion"] = _stratified(rng, _grouped(db,
        "select code, method_code from conversion where deprecated=0 and auth_name='EPSG'"),
        quotas["conversion"])
    geodetic_share = quotas["datum"] * 3 // 4
    picked["datum"] = _plain(rng, _codes(db,
        "select code from geodetic_datum where deprecated=0 and auth_name='EPSG'"), geodetic_share) + \
        _plain(rng, _codes(db,
            "select code from vertical_datum where deprecated=0 and auth_name='EPSG'"),
            quotas["datum"] - geodetic_share)
    picked["ellipsoid"] = _plain(rng, _codes(db,
        "select code from ellipsoid where deprecated=0 and auth_name='EPSG'"), quotas["ellipsoid"])
    return picked


def _proj_strings(rng, count):
    """Définitions PROJ avec +towgs84 sans code EPSG : un datum lié écrit à la main."""
    strings = []
    for _ in range(count):
        shift = ",".join(str(rng.randint(-300, 300)) for _ in range(3))
        head = rng.choice([
            f"+proj=utm +zone={rng.randint(1, 60)}",
            "+proj=longlat",
            "+proj=merc",
        ])
        strings.append(f"{head} +ellps=intl +towgs84={shift},0,0,0,0 +units=m +no_defs")
    return strings


def _bound_epsg(ref, rng, codes, count):
    """Codes EPSG géographiques 2D dont l'export PROJ porte +towgs84 (datum lié)."""
    found = []
    for code in _plain(rng, codes, len(codes)):
        if len(found) >= count:
            break
        rc, out = ref.run("projinfo", [f"EPSG:{code}", "-o", "PROJ", "-q"], [])
        if rc == 0 and any("+towgs84=" in line for line in out):
            found.append(code)
    return found


def _objects(ref, rng, db, count):
    """Une entrée d'objet : la liste d'arguments qui le désigne (sans les formats)."""
    quotas = _quotas(count)
    picked = _select(db, rng, quotas)
    objects = []
    for name in ("geog2d", "geog3d", "geocent", "proj", "vert", "compound", "engineering"):
        objects += [[f"EPSG:{code}"] for code in picked[name]]
    objects += [[f"EPSG:{code}"] for code in _bound_epsg(ref, rng, picked["bound"], quotas["bound"])]
    objects += [[text] for text in _proj_strings(rng, quotas["bound_string"])]
    for name in ("helmert", "grid", "other", "concat", "conversion"):
        objects += [["-k", "operation", f"EPSG:{code}"] for code in picked[name]]
    objects += [["-k", "datum", f"EPSG:{code}"] for code in picked["datum"]]
    objects += [["-k", "ellipsoid", f"EPSG:{code}"] for code in picked["ellipsoid"]]
    return objects


def fam_projinfo_export(gen, count):
    """Ajoute au corpus environ `count` objets, chacun dans chaque format de FORMATS."""
    db = sqlite3.connect(f"file:{gen.ref.ref}/build/data/proj.db?mode=ro", uri=True)
    objects = _objects(gen.ref, gen.rng, db, count)
    db.close()
    for index, base in enumerate(objects):
        calls = [[*base, "-o", fmt, "-q"] for fmt in FORMATS]
        if index % SINGLE_LINE_EVERY == 0:
            calls.append([*base, "-o", "WKT2:2019", "--single-line", "-q"])
        for args in calls:
            _emit(gen, args)


def _emit(gen, args):
    code, _ = gen.ref.run("projinfo", args, [])
    if code != 0:
        gen.dropped[FAMILY] += 1
        return
    gen.counts[FAMILY] += 1
    gen.cases.append({
        "id": f"{FAMILY}-{gen.counts[FAMILY]:04d}", "family": FAMILY, "tool": "projinfo",
        "args": args, "input": [], "kinds": ["text"],
    })
