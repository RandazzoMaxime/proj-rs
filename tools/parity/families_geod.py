"""Familles geod et invgeod du corpus de parité (tests/parity/geod)."""

from common import ERROR_TOKENS, wrap


GEOD_ELLPS = [
    ["+ellps=WGS84"], ["+ellps=GRS80"], ["+ellps=clrk66"], ["+ellps=intl"], ["+ellps=bessel"],
    ["+ellps=sphere"], ["+R=6371000"], ["+a=6378137", "+rf=298.257222101"],
    ["+a=6400000", "+b=6300000"],
]
GEOD_UNITS = [[], [], ["+sunits=km"], ["+sunits=us-ft"], ["+sunits=mi"], ["+sunits=m"]]
GEOD_FORMATS = [
    [], [], ["-a"], ["-p"], ["-p", "-a"], ["-f", "%.12f"], ["-a", "-f", "%.9f"],
    ["-f", "%.10f", "-F", "%.1f"], ["-F", "%.6f"], ["-F", "%.10g"], ["-w0"], ["-w2"],
    ["-W5"], ["-a", "-w4"], ["-W1", "-a"],
]
GEOD_PAIRS = ["global"] * 3 + ["antipodal", "quasi", "pole", "equator", "meridian", "parallel",
                               "antimeridian", "same", "short", "long"]
GEOD_STARTS = ["global"] * 2 + ["pole", "equator", "antimeridian"]
GEOD_ERRORS = [
    ("geod", ["+ellps=NOPE"], ["40 -74 45 1000"]),
    ("geod", ["+ellps=WGS84", "+sunits=parsec"], ["40 -74 45 1000"]),
    ("geod", ["+ellps=WGS84", "-x"], ["40 -74 45 1000"]),
    ("geod", ["+ellps=WGS84", "-f"], ["40 -74 45 1000"]),
    ("geod", ["+ellps=WGS84", "-W"], ["40 -74 45 1000"]),
    ("geod", ["+ellps=WGS84", "-wx"], ["40 -74 45 1000"]),
    ("geod", ["+ellps=WGS84"], ["abc def ghi"]),
    ("geod", ["+ellps=WGS84"], ["40 -74 45 1000 extra"]),
    ("geod", ["+ellps=WGS84"], []),
    ("geod", [], ["40 -74 45 1000"]),
    ("geod", ["+lat_1=40", "+lon_1=-74", "+ellps=WGS84"], []),
    ("geod", ["+lat_1=40", "+lon_1=-74", "+lat_2=10", "+lon_2=20", "+ellps=WGS84"], []),
    ("geod", ["+lat_1=40", "+lon_1=-74", "+A=1", "+S=10", "+n_S=0", "+ellps=WGS84"], []),
    ("geod", ["+lat_1=40", "+lon_1=-74", "+A=1", "+S=10", "+n_A=2", "+del_A=0", "+ellps=WGS84"], []),
    ("geod", ["+lat_1=40", "+lon_1=-74", "+A=1", "+S=10", "+n_A=2", "+ellps=NOPE"], []),
    ("geod", ["-I", "+ellps=WGS84"], ["40 -74 45"]),
    ("geod", ["-I", "+ellps=WGS84"], ["abc def ghi jkl"]),
    ("invgeod", ["+ellps=WGS84"], ["40 -74 40"]),
    ("invgeod", [], ["40 -74 40 -74"]),
    ("invgeod", ["+ellps=WGS84", "-f"], ["40 -74 40 -74"]),
]


def dms_text(value, positive, negative):
    """Degrés, minutes, secondes lus par dmstor : « 40d30'15.500000"N »."""
    hemisphere = positive if value >= 0 else negative
    scaled = abs(value)
    degrees = int(scaled)
    minutes_exact = (scaled - degrees) * 60
    minutes = int(minutes_exact)
    seconds = (minutes_exact - minutes) * 60
    return f"{degrees}d{minutes}'{seconds:.6f}\"{hemisphere}"


def geod_coord(rng, value, positive, negative):
    if rng.random() < 0.15:
        return dms_text(value, positive, negative)
    return f"{value:.9f}"


def geod_pair(rng):
    """Deux points (lat, lon) : antipodes, quasi-antipodes, pôles, antiméridien, distances nulles…"""
    lat1, lon1 = rng.uniform(-90, 90), rng.uniform(-180, 180)
    kind = rng.choice(GEOD_PAIRS)
    if kind == "antipodal":
        return (lat1, lon1), (-lat1, wrap(lon1 + 180))
    if kind == "quasi":
        eps = rng.choice([1e-9, 1e-7, 1e-5, 1e-3, 1e-1]) * rng.choice([-1, 1])
        return (lat1, lon1), (-lat1 + eps, wrap(lon1 + 180 + eps))
    if kind == "pole":
        return (rng.choice([90.0, -90.0]), lon1), (rng.uniform(-90, 90), rng.uniform(-180, 180))
    if kind == "equator":
        return (0.0, lon1), (0.0, rng.uniform(-180, 180))
    if kind == "meridian":
        return (lat1, lon1), (rng.uniform(-90, 90), lon1)
    if kind == "parallel":
        return (lat1, lon1), (lat1, rng.uniform(-180, 180))
    if kind == "antimeridian":
        return (lat1, rng.choice([179.9999, 179.5])), (rng.uniform(-90, 90), -179.9999)
    if kind == "same":
        return (lat1, lon1), (lat1, lon1)
    if kind == "short":
        return (lat1, lon1), (lat1 + rng.uniform(-1e-3, 1e-3), lon1 + rng.uniform(-1e-3, 1e-3))
    if kind == "long":
        return (lat1, lon1), (-lat1 + rng.uniform(-1, 1), wrap(lon1 + 180 + rng.uniform(-1, 1)))
    return (lat1, lon1), (rng.uniform(-90, 90), rng.uniform(-180, 180))


def geod_inv_line(rng):
    (lat1, lon1), (lat2, lon2) = geod_pair(rng)
    return " ".join([geod_coord(rng, lat1, "N", "S"), geod_coord(rng, lon1, "E", "W"),
                     geod_coord(rng, lat2, "N", "S"), geod_coord(rng, lon2, "E", "W")])


def geod_fwd_line(rng):
    """Point de départ, azimut, distance en mètres (ou en unités de +sunits)."""
    start = rng.choice(GEOD_STARTS)
    lat, lon = rng.uniform(-90, 90), rng.uniform(-180, 180)
    if start == "pole":
        lat = rng.choice([90.0, -90.0])
    elif start == "equator":
        lat = 0.0
    elif start == "antimeridian":
        lon = rng.choice([179.9999, -179.9999])
    azimuth = rng.choice([rng.uniform(-360, 360), rng.uniform(0, 360), 0.0, -0.0, 90.0,
                          180.0, 270.0, -90.0])
    distance = rng.choice([0.0, rng.uniform(0, 1000), rng.uniform(0, 1e6), rng.uniform(1e6, 2e7),
                           rng.uniform(2e7, 4.5e7), 20003931.458, 0.001])
    return " ".join([geod_coord(rng, lat, "N", "S"), geod_coord(rng, lon, "E", "W"),
                     f"{azimuth:.9f}", f"{distance:.9f}"])


def geod_kinds(mode, full, deg):
    """Genre de chaque champ : degrés si -f, DMS sinon (texte exact), distance en mètres."""
    angle = "deg" if deg else "dms"
    if mode == "line":
        return [angle, angle]
    if mode == "inv" and not full:
        return [angle, angle, "m"]
    if mode == "fwd" and not full:
        return [angle, angle, angle]
    return [angle] * 6 + ["m"]


def geod_options(rng):
    return rng.choice(GEOD_FORMATS), rng.choice(GEOD_ELLPS), rng.choice(GEOD_UNITS)


def geod_line_case(rng):
    """Mode ligne de geod : +lat_1 +lon_1 puis un second point, ou un azimut et une distance."""
    mode = rng.choice(["inv", "inv", "fwd", "arc"])
    lat1, lon1 = rng.uniform(-80, 80), rng.uniform(-180, 180)
    if mode == "inv":
        (la1, lo1), (la2, lo2) = geod_pair(rng)
        args = [f"+lat_1={la1:.9f}", f"+lon_1={lo1:.9f}", f"+lat_2={la2:.9f}", f"+lon_2={lo2:.9f}"]
    else:
        args = [f"+lat_1={lat1:.9f}", f"+lon_1={lon1:.9f}",
                f"+A={rng.uniform(-360, 360):.9f}", f"+S={rng.uniform(0, 2e7):.9f}"]
    if mode == "arc":
        args += [f"+n_A={rng.randint(1, 6)}", f"+del_A={rng.uniform(-90, 90):.9f}"]
    elif rng.random() < 0.5:
        args.append(f"+n_S={rng.randint(1, 12)}")
    else:
        args.append(f"+del_S={rng.uniform(1e3, 3e6):.9f}")
    return args


def emit_line_mode(gen, family, args, kinds):
    """geod en mode ligne : aucune entrée, la sortie compte autant que l'invocation."""
    code, out = gen.ref.run("geod", args, [])
    good = [o for o in out if not set(o.split()) & ERROR_TOKENS]
    if not out or any(len(o.split()) != len(kinds) for o in good):
        gen.dropped[family] += 1
        return False
    gen.counts[family] += 1
    gen.cases.append({
        "id": f"{family}-{gen.counts[family]:04d}", "family": family, "tool": "geod",
        "args": args, "input": [], "kinds": kinds,
    })
    return True


def fam_geod(gen, count):
    """Familles geod-inv, geod-fwd, invgeod-inv (5 entrées), geod-blank, geod-line, geod-err."""
    rng = gen.rng
    for _ in range(count):
        fmt, ell, unit = geod_options(rng)
        lines = [geod_inv_line(rng) for _ in range(5)]
        gen.emit("geod-inv", "geod", ["-I", *fmt, *ell, *unit], lines,
                 geod_kinds("inv", "-a" in fmt, "-f" in fmt))
    for _ in range(count):
        fmt, ell, unit = geod_options(rng)
        lines = [geod_fwd_line(rng) for _ in range(5)]
        gen.emit("geod-fwd", "geod", [*fmt, *ell, *unit], lines,
                 geod_kinds("fwd", "-a" in fmt, "-f" in fmt))
    for _ in range(count):
        fmt, ell, unit = geod_options(rng)
        lines = [geod_inv_line(rng) for _ in range(5)]
        gen.emit("invgeod-inv", "invgeod", [*fmt, *ell, *unit], lines,
                 geod_kinds("inv", "-a" in fmt, "-f" in fmt))
    for _ in range(count // 4):
        fmt, ell, unit = geod_options(rng)
        lines = [rng.choice(["", "  ", "\t"]) if rng.random() < 0.4 else geod_inv_line(rng)
                 for _ in range(5)]
        gen.emit("geod-blank", "geod", ["-I", *fmt, *ell, *unit], lines,
                 geod_kinds("inv", "-a" in fmt, "-f" in fmt))
    for _ in range(count // 2):
        fmt, ell, unit = geod_options(rng)
        args = geod_line_case(rng)
        emit_line_mode(gen, "geod-line", [*args, *fmt, *ell, *unit],
                       geod_kinds("line", False, "-f" in fmt))
    for tool, args, lines in GEOD_ERRORS:
        gen.counts["geod-err"] += 1
        gen.cases.append({
            "id": f"geod-err-{gen.counts['geod-err']:04d}", "family": "geod-err", "tool": tool,
            "args": args, "input": lines, "kinds": [],
        })
