"""Familles `cct` et `proj` du corpus de parité."""

from common import valid_line
from reference import projection_defs


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
