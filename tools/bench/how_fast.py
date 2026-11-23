#!/usr/bin/env python3
"""Rewrite the README "How fast" block from the newest bench line that carries libproj times.

The block between `<!-- bench-how-fast:start -->` and `<!-- bench-how-fast:end -->` is built
from one line of bench/history.jsonl, so the numbers in the README are that line's. Standard
library only.
"""
import math

import history

PRIMARY = "gha-ubuntu-24.04-x86_64"
ARM_MACHINES = [
    ("gha-macos-arm64-m1", "Apple M1"),
    ("gha-ubuntu-24.04-arm64", "Neoverse N2"),
]

MACHINE_NOTES = {
    "cloud-4c-avx512": (
        "a 4-vCPU cloud VM (Intel Xeon at 2.10 GHz), the process pinned to one core"
    ),
    PRIMARY: "a GitHub Actions runner (Linux x86-64, 4 vCPU), the process pinned to one core",
    "gha-ubuntu-24.04-arm64": (
        "a GitHub Actions runner (Linux ARM, Neoverse N2, 4 vCPU), the process pinned to one core"
    ),
    "gha-macos-arm64-m1": "a GitHub Actions runner (macOS, Apple M1, virtual), not pinned",
}

PATH_NOTES = {
    "avx2": (
        "The batch kernels take the AVX2 path, which every x86-64 CPU has since 2013 (Intel) "
        "and 2015 (AMD); AVX-512 is not used."
    ),
    "neon": "The batch kernels take the NEON path, which every aarch64 CPU has.",
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
    return f"about the same, {ratio:.2f}×"


def than(ratio):
    text = speedup_text(ratio)
    return "as fast as" if text.startswith("about") else f"{text} than"


def ns(value):
    return f"{value:.3g} ns" if value < 1000 else f"{value:,.0f} ns"


def geomean(values):
    return math.exp(sum(math.log(v) for v in values) / len(values))


def complete(row):
    return all(f"facade_convert_{p}" in row["libproj_ns"] for p in PAIRS)


def pick(rows, machine=PRIMARY):
    """The newest line of `machine` with libproj times, else the newest of any machine."""
    for row in reversed(rows):
        if row["machine"] == machine and complete(row):
            return row
    for row in reversed(rows):
        if complete(row):
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
    low, high = min(ratios), max(ratios)
    if low >= 1.05:
        return f"{speedup_text(geomean(ratios))} on average, from {low:.1f}× to {high:.1f}×"
    return (
        f"{speedup_text(geomean(ratios))} on average, from {speedup_text(low)} "
        f"to {speedup_text(high)}"
    )


def extremes_text(ratios):
    low, high = min(ratios), max(ratios)
    if low >= 1.05:
        return f"faster in every case, from {low:.1f}× to {high:.1f}×"
    return f"the slowest case is {speedup_text(low)}, the fastest {speedup_text(high)}"


def body(row):
    cases = rows_of(row)
    ratios = [c[4] for c in cases]
    single = [c[4] for c in cases if "one point" in c[1]]
    array = [c[4] for c in cases if "array" in c[1] or "200,000" in c[1]]
    note = MACHINE_NOTES.get(row["machine"], "")
    rounds = row["result"].get("rounds", 1)
    where = note or "the bench machine"
    version = row["result"].get("version", "1")
    path = PATH_NOTES.get(row["result"].get("batch_path", ""), "")
    lines = [
        f"These numbers are proj-rs {version}'s, measured by the CI.",
        "",
        f"On {where}, proj-rs is {than(geomean([c[4] for c in cases]))} PROJ 9.9.0's `libproj` on "
        f"average over the ten cases below (geometric mean of the ratios; {extremes_text(ratios)}). "
        f"One point at a time, proj-rs is {spread_text(single)}. On a slice of points, it is "
        f"{spread_text(array)}. "
        "This is one machine: the ratios on another CPU, or on a build of libproj with other "
        "flags, will differ. " + path,
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
        "With no grid, NAD27 → WGS 84 is PROJ's ballpark offset on both sides, a change of units and "
        "axis order with no datum shift, which proj-rs applies as one step. "
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
    lines += arm_section(history.load())
    return "\n".join(lines)


def arm_section(rows):
    """A table of the same conversions on the ARM machines of the history, if it has any."""
    found = []
    for machine, name in ARM_MACHINES:
        row = pick([r for r in rows if r["machine"] == machine], machine)
        if row["machine"] == machine:
            found.append((name, row))
    if not found:
        return []
    names = " and ".join(f"{name} ({MACHINE_NOTES[row['machine']]})" for name, row in found)
    head = "| Conversion | Call |" + "".join(f" PROJ 9.9.0, {n} | proj-rs, {n} |" for n, _ in found)
    out = [
        "",
        "### On ARM",
        "",
        f"The same ten cases on {names}. The batch kernels take the NEON path; "
        "the ratio is PROJ 9.9.0's time over proj-rs's.",
        "",
        head,
        "|---|---|" + "---:|---:|" * len(found),
    ]
    for pair in PAIRS:
        for case, calls in (
            (f"facade_convert_{pair}", "`convert`"),
            (f"facade_convert_array_{pair}", "`convert_array`"),
        ):
            cells = ""
            for _, row in found:
                theirs, ours = row["libproj_ns"][case], row["ns"][case]
                cells += f" {ns(theirs)} | {ns(ours)} ({speedup_text(theirs / ours)}) |"
            out.append(f"| {label(pair)} | {calls} |{cells}")
    out += [""]
    for name, row in found:
        ratios = [c[4] for c in rows_of(row)]
        out.append(
            f"On {name}, proj-rs is {than(geomean(ratios))} PROJ 9.9.0 on average over the ten cases "
            f"({extremes_text(ratios)}); line of {history.day_time(row['date'])} UTC at commit "
            f"`{row['commit']}`, the best of {row['result'].get('rounds', 1)} runs.\n"
        )
    return out


def main():
    row = pick(history.load())
    history.write_block("bench-how-fast", body(row))


if __name__ == "__main__":
    main()
