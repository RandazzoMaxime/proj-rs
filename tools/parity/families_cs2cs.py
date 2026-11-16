"""Familles `cs2cs` et `projinfo` du corpus de parité."""

from common import cs2cs_kinds, datum_label
from generator import attempts, emit_text


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
