"""Constantes et géométrie partagées par le générateur de parité."""

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
