"""Famille réseau : paires dont l'opération utilise une grille du CDN, réseau ON.

Les grilles viennent de `grid_alternatives` (nom CDN, `url` non nul) de la proj.db de la
référence. Une paire est retenue si la première opération de `projinfo` (réseau ON, sans grille
manquante) n'utilise que des grilles du CDN. Chaque paire donne quatre invocations `cs2cs` :
défaut, `--only-best`, `--only-best=no`, `--no-ballpark`. Huit points : moitié dans l'emprise
de l'opération, moitié dans le monde (souvent hors grille).
"""

from common import cs2cs_kinds
from families_grids import WORLD_EXT, chosen_proj_string, crs_members, grid_names

NETWORK_MODES = {
    "net-default": [],
    "net-only-best": ["--only-best"],
    "net-only-best-no": ["--only-best=no"],
    "net-no-ballpark": ["--no-ballpark"],
}


def network_pairs(gen, op, cdn_names, cache):
    """Couples (source, cible) dont `projinfo` (réseau ON) choisit une grille du CDN."""
    sources = crs_members(gen, op["src"], op["ext"])
    targets = crs_members(gen, op["dst"], op["ext"])
    if not sources or not targets:
        return []
    pairs = []
    for _ in range(4):
        a, b = gen.rng.choice(sources), gen.rng.choice(targets)
        if a is b or (a["ref"], b["ref"]) in cache:
            continue
        _, lines = gen.ref.run("projinfo", ["-s", a["ref"], "-t", b["ref"], "-o", "PROJ"], [])
        names = grid_names(chosen_proj_string(lines))
        ok = bool(names) and all(name in cdn_names for name in names)
        cache[(a["ref"], b["ref"])] = ok
        if ok:
            pairs.append((a, b))
    return pairs


def fam_network(gen, candidates, pair_count, points):
    """`pair_count` paires, quatre modes chacune ; l'ordre des opérations suit la graine."""
    cdn_names = {name for op in candidates for name in op["files"]}
    ops = list(candidates)
    gen.rng.shuffle(ops)
    cache = {}
    done = 0
    for op in ops:
        for src, dst in network_pairs(gen, op, cdn_names, cache):
            if done >= pair_count:
                return done
            inside = gen.sample(src, points // 2, ext=op["ext"])
            outside = gen.sample(src, points - points // 2, ext=WORLD_EXT)
            lines = inside + outside
            if not lines:
                continue
            done += 1
            for family, options in NETWORK_MODES.items():
                args = ["-d", "12", *options, src["ref"], "+to", dst["ref"]]
                gen.emit(family, "cs2cs", args, lines, cs2cs_kinds(dst))
    return done
