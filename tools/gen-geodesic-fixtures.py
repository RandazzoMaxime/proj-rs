#!/usr/bin/env python3
"""Fixtures of tests/geodesic_reference.rs: what the `invgeod` and `geod` of PROJ 9.9.0 print.

    tools/build-reference.sh                       # builds geod and invgeod 9.9.0
    tools/gen-geodesic-fixtures.py target/proj-reference/build/bin

Each line of `inverse.txt` is `ellipsoid lat1 lon1 lat2 lon2 azi1 azi2 s12`, each line of
`direct.txt` is `ellipsoid lat1 lon1 azi1 s12 lat2 lon2 azi2`. Angles are degrees, lengths metres.
The inputs are written as PROJ read them. `geod` refuses a negative flattening, so a prolate
ellipsoid has no line here.
"""
import random
import subprocess
import sys
from pathlib import Path

ELLIPSOIDS = {
    "wgs84": "+ellps=WGS84",
    "grs80": "+ellps=GRS80",
    "bessel": "+ellps=bessel",
    "sphere": "+a=6371000 +f=0",
    "flat20": "+a=6378137 +f=0.05",
    "flat300": "+a=6378137 +f=0.0033528106647474805",
}


def run(binary: Path, ellipsoid: str, rows: list[str], formats: list[str]) -> list[list[str]]:
    out = subprocess.run(
        [str(binary), *ellipsoid.split(), *formats],
        input="\n".join(rows) + "\n",
        capture_output=True,
        text=True,
        check=True,
    ).stdout.splitlines()
    return [line.split() for line in out]


def fmt(x: float) -> str:
    return f"{x:.9f}".rstrip("0").rstrip(".") if x != int(x) else str(int(x))


def inverse_inputs(rng: random.Random) -> list[tuple[str, str, str, str]]:
    cases = []
    specials = [
        (0, 0, 0, 90), (0, 0, 0, 179), (0, 0, 0.5, 179.7), (0, 0, 0, 180), (0, 0, 0.5, 179.5),
        (45, 10, -45, 190), (45, 10, 45, 10), (10, 20, 10, 20), (90, 0, -90, 0), (90, 0, 10, 45),
        (-90, 0, 30, 100), (10, 170, 10, -170), (10, 10, 60, 10), (60, 10, -50, 10), (0, 0, 80, 0),
        (30, 0, 30, 90), (30, 0, -30, 90), (20.001, 0, 20.001, 0), (52.784459512564, 0, -52.784459512563990912, 179.634407464943777557),
        (48.522876735459, 0, -48.52287673545898293, 179.599720456223079643), (89.9999, 0, 89.9999, 180),
        (0.0001, 0, 0.0001, 0.0002), (40.6, -73.8, 51.5, -0.5), (35.5, 139.7, -33.9, 151.2), (-33.9, 18.4, 64.1, -21.9),
    ]
    for s in specials:
        cases.append(tuple(str(v) for v in s))
    for _ in range(70):
        cases.append((fmt(rng.uniform(-90, 90)), fmt(rng.uniform(-180, 180)),
                      fmt(rng.uniform(-90, 90)), fmt(rng.uniform(-180, 180))))
    for _ in range(10):
        lat = rng.uniform(-60, 60)
        lon = rng.uniform(-170, 170)
        cases.append((fmt(lat), fmt(lon), fmt(lat + rng.uniform(-1e-3, 1e-3)), fmt(lon + rng.uniform(-1e-3, 1e-3))))
    for _ in range(10):
        lat = rng.uniform(-80, 80)
        lon = rng.uniform(-90, 90)
        cases.append((fmt(lat), fmt(lon), fmt(-lat + rng.uniform(-0.2, 0.2)), fmt(lon + 180 + rng.uniform(-0.5, 0.5))))
    return cases


def direct_inputs(rng: random.Random) -> list[tuple[str, str, str, str]]:
    cases = []
    specials = [
        (0, 0, 0, 1000000), (0, 0, 90, 1000000), (0, 0, 180, 1000000), (0, 0, -90, 5000000),
        (0, 0, 90, 20003931), (0, 0, 90, 40075016), (45, 10, 45, 0), (90, 0, 180, 1000000),
        (-90, 0, 0, 1000000), (10, 20, 33, 19900000), (10, 20, 33, 20100000), (50, 5, 0, 10001965),
        (30, 60, 270, 12345678), (-30, -60, 135, 987654), (0.0001, 0, 90, 0.01),
    ]
    for s in specials:
        cases.append(tuple(str(v) for v in s))
    for _ in range(70):
        cases.append((fmt(rng.uniform(-90, 90)), fmt(rng.uniform(-180, 180)),
                      fmt(rng.uniform(-180, 180)), fmt(rng.uniform(0, 20000000))))
    for _ in range(15):
        cases.append((fmt(rng.uniform(-80, 80)), fmt(rng.uniform(-180, 180)),
                      fmt(rng.uniform(-180, 180)), fmt(rng.uniform(0, 100))))
    return cases


def main() -> None:
    bins = Path(sys.argv[1])
    out = Path(__file__).resolve().parent.parent / "tests" / "geodesic"
    rng = random.Random(20260918)
    inverse_cases = inverse_inputs(rng)
    direct_cases = direct_inputs(rng)
    lines = []
    for name, ellipsoid in ELLIPSOIDS.items():
        rows = [" ".join(c) for c in inverse_cases]
        for case, result in zip(inverse_cases, run(bins / "invgeod", ellipsoid, rows, ["-f", "%.15f", "-F", "%.9f"])):
            lines.append(" ".join([name, *case, *result]))
    (out / "inverse.txt").write_text("\n".join(lines) + "\n")
    lines = []
    for name, ellipsoid in ELLIPSOIDS.items():
        rows = [" ".join(c) for c in direct_cases]
        for case, result in zip(direct_cases, run(bins / "geod", ellipsoid, rows, ["-f", "%.15f"])):
            lines.append(" ".join([name, *case, *result]))
    (out / "direct.txt").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
