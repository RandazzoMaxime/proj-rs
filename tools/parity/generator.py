"""Générateur d'invocations : points, paires, écriture des cas acceptés."""

from collections import defaultdict

from common import ERROR_TOKENS, lon_width, overlap, valid_line, wrap


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


def attempts(want, make):
    """Appelle `make()` jusqu'à `want` invocations retenues, avec une borne d'essais."""
    made = 0
    for _ in range(want * 25):
        if made >= want:
            break
        if make():
            made += 1
    return made
