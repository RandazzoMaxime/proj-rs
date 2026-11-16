"""Familles aléatoires du différentiel (`fuzz_diff.py`) : projections, chaînes `cs2cs` et pipelines
`cct` aux paramètres tirés au hasard. La référence écarte ce qu'elle refuse."""

import re

from common import valid_line
from generator import attempts
from reference import projection_defs

ELLIPSOIDS = ["WGS84", "GRS80", "intl", "bessel", "clrk66", "clrk80", "krass", "airy", "sphere",
              "evrst30", "mod_airy", "WGS72", "aust_SA", "helmert", "NWL9D", "GRS67"]
UNITS = ["km", "us-ft", "ft", "link", "ch"]
# Projections tirées pour les familles `cs2cs` et `cct` : celles dont l'inverse existe presque partout.
CLASSIC = ["merc", "tmerc", "lcc", "aea", "eqdc", "stere", "laea", "poly", "cass", "sinu", "eqc",
           "omerc", "ortho", "gnom", "aeqd", "moll", "robin", "utm", "etmerc", "somerc", "krovak"]
ELLIPSOID_PROJECTIONS = {"utm", "tmerc", "etmerc", "somerc", "krovak", "cass", "poly", "merc", "lcc"}


def params_of(definition):
    """`+k=v` d'une définition, dans l'ordre, en dictionnaire."""
    params = {}
    for token in definition.split():
        key, _, value = token.lstrip("+").partition("=")
        params[key] = value
    return params


def render(params):
    return " ".join(f"+{key}" if value == "" else f"+{key}={value}" for key, value in params.items())


def projection_keys(reference):
    """Paramètres propres à chaque projection, lus dans `proj -lP` de la référence.

    Un paramètre que la projection ne lit pas est ignoré par PROJ et refusé par le port
    (`UnknownKeys::Refuse`, écart consigné dans docs/FUZZ.md) : le tirage s'en tient aux
    paramètres listés, pour que le différentiel cherche d'autres écarts.
    """
    _, lines = reference.run("proj", ["-lP"], [])
    keys, current = {}, None
    for line in lines:
        if not line.startswith("\t"):
            current = line.split(" :")[0]
            keys[current] = set()
        elif current is not None:
            keys[current] |= {word.rstrip("=") for word in line.split() if re.fullmatch(r"\w+=?", word)}
    return keys


def random_extras(rng, params, allowed):
    """Complète `params` : un paramètre déjà présent est remplacé, jamais écrit deux fois.

    `allowed` : les paramètres que la projection lit en plus des communs.
    """
    params["ellps"] = rng.choice(ELLIPSOIDS)
    if rng.random() < 0.8:
        params["lon_0"] = f"{rng.uniform(-180, 180):.6f}"
    if rng.random() < 0.6:
        params["lat_0"] = f"{rng.uniform(-60, 60):.6f}"
    for key in ("lat_1", "lat_2", "lat_ts"):
        if key in allowed and rng.random() < 0.7:
            params[key] = f"{rng.uniform(-70, 70):.6f}"
    if rng.random() < 0.3:
        params["k_0"] = f"{rng.uniform(0.98, 1.02):.8f}"
    if rng.random() < 0.4:
        params["x_0"] = f"{rng.uniform(-1e6, 1e6):.1f}"
        params["y_0"] = f"{rng.uniform(-1e6, 1e6):.1f}"
    if rng.random() < 0.08:
        params["units"] = rng.choice(UNITS)
    if rng.random() < 0.05:
        params["axis"] = rng.choice(["neu", "wsu", "esu", "wnu"])
    if rng.random() < 0.04:
        params["over"] = ""
    return params


def random_points(rng, count, lon_0=None):
    near = [(rng.gauss(lon_0 or 0.0, 25.0), rng.uniform(-80, 80)) for _ in range(count // 2)]
    world = [(rng.uniform(-180, 180), rng.uniform(-89, 89)) for _ in range(count - len(near))]
    return [f"{lon:.9f} {lat:.9f}" for lon, lat in near + world]


def fam_random_proj(gen, want, points=8):
    """`proj` et `proj -I` : une projection au hasard, des paramètres au hasard, des points au hasard."""
    defs = projection_defs(gen.ids)
    allowed = projection_keys(gen.ref)
    rng = gen.rng

    def make():
        pid = rng.choice(gen.ids)
        params = random_extras(rng, params_of(defs[pid]), allowed.get(pid, set()))
        params["proj"] = pid
        args = ["-f", "%.12f", *render(params).split()]
        lines = random_points(rng, points, float(params.get("lon_0", 0)))
        if rng.random() < 0.5:
            return gen.emit("random-proj-fwd", "proj", args, lines, ["m", "m"])
        _, forward = gen.ref.run("proj", args, lines)
        good = [o for o in forward if valid_line(o, 2)]
        return len(good) >= 3 and gen.emit("random-proj-inv", "proj", ["-I", *args], good,
                                           ["deg", "deg"])
    return attempts(want, make)


def datum_part(rng, named):
    kind = rng.random()
    if named and kind < 0.3:
        return {"datum": rng.choice(["WGS84", "NAD83", "NAD27", "potsdam", "carthage", "GGRS87",
                                     "hermannskogel", "ire65", "nzgd49", "OSGB36"])}
    params = {"ellps": rng.choice(ELLIPSOIDS)}
    if kind < 0.8:
        shift = [rng.uniform(-300, 300) for _ in range(3)]
        if rng.random() < 0.4:
            shift += [rng.uniform(-3, 3) for _ in range(3)] + [rng.uniform(-5, 5)]
        params["towgs84"] = ",".join(f"{v:.4f}" for v in shift)
    return params


def random_crs(rng, projected, allowed, named):
    params = {"proj": "longlat"}
    if projected:
        name = rng.choice(CLASSIC)
        params = random_extras(rng, {}, allowed.get(name, set()))
        params.pop("ellps", None)
        params["proj"] = name
        if params["proj"] == "utm":
            for key in ("lon_0", "lat_0", "lat_1", "lat_2", "k_0", "x_0", "y_0"):
                params.pop(key, None)
            params["zone"] = str(rng.randint(1, 60))
        elif params["proj"] == "krovak":
            params.pop("lat_0", None)
    params.update(datum_part(rng, named))
    return params


def fam_random_cs2cs(gen, want, points=8, named_datums=False):
    """`cs2cs` entre deux chaînes PROJ (géographique ou projetée), précision et sens au hasard.

    `named_datums` ajoute `+datum=NAD83`, `NAD27`, `carthage`… : le port s'écarte de PROJ sur les
    paires où un côté est un datum nommé et l'autre un ellipsoïde avec `+towgs84` (docs/FUZZ.md).
    """
    rng = gen.rng
    allowed = projection_keys(gen.ref)

    def make():
        src = random_crs(rng, rng.random() < 0.3, allowed, named_datums)
        dst = random_crs(rng, rng.random() < 0.7, allowed, named_datums)
        precision = rng.choice(["9", "10", "12"])
        reverse = ["-I"] if rng.random() < 0.15 else []
        lon_0 = float(dst.get("lon_0", 0))
        lines = random_points(rng, points, lon_0)
        if src["proj"] != "longlat":
            return False
        args = ["-d", precision, *reverse, *render(src).split(), "+to", *render(dst).split()]
        kinds = ["deg", "deg", "m"] if dst["proj"] == "longlat" else ["m", "m", "m"]
        if reverse:
            kinds = ["deg", "deg", "m"]
        return gen.emit("random-cs2cs", "cs2cs", args, lines, kinds)
    return attempts(want, make)


def random_helmert(rng):
    params = {"proj": "helmert", "x": f"{rng.uniform(-500, 500):.4f}",
              "y": f"{rng.uniform(-500, 500):.4f}", "z": f"{rng.uniform(-500, 500):.4f}"}
    if rng.random() < 0.6:
        params.update(rx=f"{rng.uniform(-5, 5):.5f}", ry=f"{rng.uniform(-5, 5):.5f}",
                      rz=f"{rng.uniform(-5, 5):.5f}", s=f"{rng.uniform(-20, 20):.5f}")
        params["convention"] = rng.choice(["position_vector", "coordinate_frame"])
    if rng.random() < 0.3:
        params.update(dx=f"{rng.uniform(-0.1, 0.1):.5f}", dy=f"{rng.uniform(-0.1, 0.1):.5f}",
                      dz=f"{rng.uniform(-0.1, 0.1):.5f}", t_epoch=f"{rng.uniform(2000, 2020):.1f}")
    if rng.random() < 0.15:
        params["exact"] = ""
    return render(params)


def random_pipeline(rng, allowed):
    """Chaîne cohérente en unités : l'état est angulaire, cartésien ou plan. Rend (texte, état final)."""
    state, steps = "angular", []
    for _ in range(rng.randint(2, 5)):
        if state == "angular":
            move = rng.choice(["cart", "proj", "geoc", "molodensky", "axisswap"])
            ell = f"+ellps={rng.choice(ELLIPSOIDS)}"
            if move == "cart":
                steps.append(f"+proj=cart {ell}")
                state = "cart"
            elif move == "proj":
                name = rng.choice(CLASSIC)
                params = random_extras(rng, {}, allowed.get(name, set()))
                params.pop("units", None)
                params.pop("axis", None)
                params["proj"] = name
                if params["proj"] == "utm":
                    params = {"proj": "utm", "zone": str(rng.randint(1, 60)),
                              "ellps": params["ellps"]}
                steps.append(render(params))
                state = "plane"
            elif move == "geoc":
                steps.append(f"+proj=geoc {ell}")
            elif move == "molodensky":
                shift = " ".join(f"+d{axis}={rng.uniform(-300, 300):.3f}" for axis in "xyz")
                steps.append(f"+proj=molodensky {ell} +da={rng.uniform(-300, 300):.2f} "
                             f"+df={rng.uniform(-1e-4, 1e-4):.9f} {shift}"
                             + (" +abridged" if rng.random() < 0.5 else ""))
            else:
                steps.append("+proj=axisswap +order=2,1")
        elif state == "cart":
            if rng.random() < 0.7:
                steps.append(random_helmert(rng))
            else:
                steps.append(f"+inv +proj=cart +ellps={rng.choice(ELLIPSOIDS)}")
                state = "angular"
        else:
            move = rng.choice(["unitconvert", "axisswap", "stop"])
            if move == "unitconvert":
                steps.append("+proj=unitconvert +xy_in=m +xy_out=km")
                steps.append("+proj=unitconvert +xy_in=km +xy_out=m")
            elif move == "axisswap":
                steps.append("+proj=axisswap +order=2,1,3")
            else:
                break
    text = " ".join(f"+step {step}" for step in steps)
    return f"+proj=pipeline {text}", state


def fam_random_cct(gen, want, points=6):
    """`cct` : pipelines aléatoires, entrées 4D aléatoires, sens direct ou `-I`."""
    rng = gen.rng
    allowed = projection_keys(gen.ref)

    def make():
        pipeline, state = random_pipeline(rng, allowed)
        lines = [f"{line} {rng.uniform(-100, 4000):.3f} {rng.uniform(2000, 2025):.2f}"
                 for line in random_points(rng, points)]
        inverse = rng.random() < 0.1
        out_angular = (state == "angular") != inverse
        kinds = ["deg", "deg", "m", "t"] if out_angular else ["m", "m", "m", "t"]
        if inverse and state == "cart":
            kinds = ["m", "m", "m", "t"]
        args = ["-d", rng.choice(["9", "10", "12"]), *(["-I"] if inverse else []), *pipeline.split()]
        return gen.emit("random-cct", "cct", args, lines, kinds)
    return attempts(want, make)
