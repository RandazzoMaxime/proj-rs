#!/usr/bin/env python3
"""Rewrite the README "How fast" block from the newest bench line that carries libproj times.

The block between `<!-- bench-how-fast:start -->` and `<!-- bench-how-fast:end -->` is built
from one line of bench/history.jsonl, so the numbers in the README are that line's. Standard
library only.
"""
import math

import history

MACHINE_NOTES = {
    "cloud-4c-avx512": (
        "a 4-vCPU cloud VM (Intel Xeon at 2.10 GHz with AVX-512; the batch kernels take the AVX2 "
        "path), the process pinned to one core"
    ),
}

CRS_NAMES = {
    "4326": "WGS 84",
    "2154": "RGF93 / Lambert-93",
    "32631": "WGS 84 / UTM 31N",
    "3857": "Web Mercator",
    "4267": "NAD27",
}

PAIRS = ["4326_2154", "4326_32631", "4326_3857", "2154_32631", "4267_4326"]
BARE = [
    ("utm31_forward", "UTM 31 forward, `+proj=utm`"),
    ("lambert93_forward", "Lambert-93 forward, `+proj=lcc`"),
    ("webmerc_forward", "Web Mercator forward, `+proj=merc`"),
]


def speedup_text(ratio):
    """`ratio` is libproj's time over ours."""
    if ratio >= 1.05:
        return f"{ratio:.1f}× faster"
    if ratio <= 1 / 1.05:
        return f"{1 / ratio:.1f}× slower"
    return "about the same speed"


def than(ratio):
    text = speedup_text(ratio)
    return "as fast as" if text.startswith("about") else f"{text} than"


def ns(value):
    return f"{value:.3g} ns" if value < 1000 else f"{value:,.0f} ns"


def geomean(values):
    return math.exp(sum(math.log(v) for v in values) / len(values))


def pick(rows):
    for row in reversed(rows):
        if all(f"facade_convert_{p}" in row["libproj_ns"] for p in PAIRS):
            return row
    raise SystemExit("how_fast.py: no line of the history has libproj times for the facade cases")


def label(pair):
    source, target = pair.split("_")
    return f"{CRS_NAMES[source]} → {CRS_NAMES[target]} (EPSG:{source} → {target})"


def rows_of(row):
    out = []
    for pair in PAIRS:
        for case, calls in (
            (f"facade_convert_{pair}", "`convert`, one point at a time"),
            (f"facade_convert_array_{pair}", "`convert_array`, 200,000 points"),
        ):
            theirs, ours = row["libproj_ns"][case], row["ns"][case]
            out.append((pair, calls, theirs, ours, theirs / ours))
    return out


def spread_text(ratios):
    return (
        f"{speedup_text(geomean(ratios))} on average, from {speedup_text(min(ratios))} "
        f"to {speedup_text(max(ratios))}"
    )


def body(row):
    cases = rows_of(row)
    single = [c[4] for c in cases if "one point" in c[1]]
    array = [c[4] for c in cases if "array" in c[1] or "200,000" in c[1]]
    note = MACHINE_NOTES.get(row["machine"], "")
    rounds = row["result"].get("rounds", 1)
    where = f"`{row['machine']}`" + (f", {note}" if note else "")
    lines = [
        "These numbers are PROJ-RS v1's. The optimisation work (wider SIMD, GPU) comes in the "
        "versions after it.",
        "",
        f"On {where}, proj-rs is {than(geomean([c[4] for c in cases]))} PROJ 9.9.0's `libproj` on "
        f"average over the ten cases below (geometric mean of the ratios; the slowest case is "
        f"{speedup_text(min(c[4] for c in cases))}, the fastest {speedup_text(max(c[4] for c in cases))}). "
        f"One point at a time, proj-rs is {spread_text(single)}. On a slice of points, it is "
        f"{spread_text(array)}. "
        "This is one machine: the ratios on another CPU, or on a build of libproj with other "
        "flags, will differ.",
        "",
        "| Conversion | Call | PROJ 9.9.0 | proj-rs |",
        "|---|---|---:|---:|",
    ]
    for pair, calls, theirs, ours, ratio in cases:
        lines.append(f"| {label(pair)} | {calls} | {ns(theirs)} | {ns(ours)} ({speedup_text(ratio)}) |")
    when = history.day_time(row["date"])
    best = f", the best of {rounds} runs for each case" if rounds > 1 else ""
    lines += [
        "",
        f"Nanoseconds per point; lower is faster. Line of {when} UTC at commit `{row['commit']}`{best}. "
        "Both sides convert the same points, and the bench stops if their outputs differ. "
        "What exactly is compared, and how to rerun it: [docs/BENCHMARK.md](docs/BENCHMARK.md).",
        "",
        "<details>",
        "<summary><b>Bare projections against libproj, and the ten slowest operations</b></summary>",
        "",
        "<br>",
        "",
        "Bare projections, forward, from `+proj=` strings (no CRS lookup), one point at a time "
        "over a million points:",
        "",
        "| Projection | PROJ 9.9.0 | proj-rs |",
        "|---|---:|---:|",
    ]
    for case, name in BARE:
        theirs, ours = row["libproj_ns"][case], row["ns"][case]
        lines.append(f"| {name} | {ns(theirs)} | {ns(ours)} ({speedup_text(theirs / ours)}) |")
    slowest = sorted(
        ((k[3:], v) for k, v in row["ns"].items() if k.startswith("op_")), key=lambda kv: -kv[1]
    )[:10]
    lines += [
        "",
        "Ten slowest operations of the catalogue (same line, a 128-point grid, no libproj column):",
        "",
        "| Operation | ns/point |",
        "|---|---:|",
    ]
    lines += [f"| `{name}` | {value:.1f} |" for name, value in slowest]
    lines += ["", "</details>"]
    return "\n".join(lines)


def main():
    row = pick(history.load())
    history.write_block("bench-how-fast", body(row))


if __name__ == "__main__":
    main()
