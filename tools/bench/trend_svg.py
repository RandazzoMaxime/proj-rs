#!/usr/bin/env python3
"""Draw bench/trend-<machine>-{light,dark}.svg from bench/history.jsonl.

One chart pair per machine that has at least two lines: a number compares with its own
machine's only. The README block between `<!-- bench-trend:start -->` and `<!-- bench-trend:end -->`
is rewritten with the pictures and the first and latest value of every series. The output depends
on the history alone, so a second run changes nothing. Standard library only.
"""
import math
from html import escape
from textwrap import wrap

import history

SERIES = [
    ("utm31_forward", "UTM forward"),
    ("lambert93_forward", "Lambert-93 forward"),
    ("webmerc_forward", "Web Mercator forward"),
    ("facade_convert_array_4326_2154", "convert_array 4326→2154"),
    ("helmert7_apply", "Helmert"),
]

THEMES = {
    "light": {
        "text": "#1f2328",
        "dim": "#59636e",
        "grid": "#d1d9e0",
        "ring": "#ffffff",
        "colors": ["#0969da", "#cf4a14", "#14805a", "#9a6700", "#bf3989"],
    },
    "dark": {
        "text": "#f0f6fc",
        "dim": "#9198a1",
        "grid": "#262c36",
        "ring": "#0d1117",
        "colors": ["#3987e5", "#d95926", "#199e70", "#c98500", "#d55181"],
    },
}

WIDTH = 800
LEFT, RIGHT = 70, 548
TOP_PAD = 14
PLOT_H = 220
FONT = "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif"


def text_width(text, size):
    return len(text) * size * 0.56


def f(value):
    return f"{value:.1f}".rstrip("0").rstrip(".")


def ticks(low, high):
    """Round values on a logarithmic axis that cover [low, high]."""
    steps = [1, 3] if high / low > 12 else [1, 2, 5]
    out = []
    for exponent in range(math.floor(math.log10(low)) - 1, math.ceil(math.log10(high)) + 2):
        for step in steps:
            out.append(step * 10.0**exponent)
    below = [t for t in out if t <= low]
    above = [t for t in out if t >= high]
    return [t for t in out if below[-1] <= t <= above[0]]


def spread(ys, gap, low, high):
    """Move label baselines apart so that neighbours keep `gap`, inside [low, high]."""
    order = sorted(range(len(ys)), key=lambda i: ys[i])
    placed = [ys[i] for i in order]
    for k in range(1, len(placed)):
        placed[k] = max(placed[k], placed[k - 1] + gap)
    if placed and placed[-1] > high:
        placed[-1] = high
        for k in range(len(placed) - 2, -1, -1):
            placed[k] = min(placed[k], placed[k + 1] - gap)
    out = [0.0] * len(ys)
    for slot, i in enumerate(order):
        out[i] = max(low, placed[slot])
    return out


def series_for(rows):
    """For each series with a value on this machine: the points (run index, ns, date)."""
    out = []
    for position, (case, label) in enumerate(SERIES):
        points = [(i, row["ns"][case], row["date"]) for i, row in enumerate(rows) if case in row["ns"]]
        if points:
            out.append((position, label, points))
    return out


def title_of():
    return "Time per point"


def subtitle_of():
    text = (
        "Nanoseconds per point, from a million-point grid for the bare projections and the "
        "Helmert shift, and from a 200,000-point slice for convert_array through the facade; "
        "the median of 3 trials of 11 passes. Lower is faster. "
        "A gap is a run from before that case existed."
    )
    return wrap(text, 118)


def render(machine, rows, theme):
    palette = THEMES[theme]
    series = series_for(rows)
    values = [v for _, _, points in series for _, v, _ in points]
    ticks_list = ticks(min(values), max(values))
    lo, hi = math.log10(ticks_list[0]), math.log10(ticks_list[-1])
    subtitle = subtitle_of()

    y = 26
    out = []
    title = title_of()
    out.append(f'<text x="16" y="{y}" font-size="17" class="b">{escape(title)}</text>')
    y += 19
    for line in subtitle:
        out.append(f'<text x="16" y="{y}" font-size="13" class="d">{escape(line)}</text>')
        y += 19
    y += 1

    legend_x, legend_y = 16.0, y
    legend = []
    for position, label, _ in series:
        need = 22 + text_width(label, 12.5) + 20
        if legend_x + need > WIDTH - 16 and legend_x > 16:
            legend_x, legend_y = 16.0, legend_y + 20
        legend.append(f'<path d="M{f(legend_x)} {f(legend_y)}h16" class="l s{position}"/>')
        legend.append(
            f'<text x="{f(legend_x + 22)}" y="{f(legend_y + 4)}" font-size="12.5">{escape(label)}</text>'
        )
        legend_x += need
    out.extend(legend)
    y = legend_y + 26
    out.append(f'<text x="16" y="{f(y)}" font-size="11.5" class="d">nanoseconds per point, logarithmic</text>')
    top = y + TOP_PAD
    bottom = top + PLOT_H

    def ypos(value):
        return bottom - (math.log10(value) - lo) / (hi - lo) * PLOT_H

    count = len(rows)
    inner = (RIGHT - LEFT) - 12

    def xpos(i):
        return LEFT + 6 + (inner * i / (count - 1) if count > 1 else inner / 2)

    for tick in ticks_list:
        out.append(f'<line x1="{LEFT - 7}" y1="{f(ypos(tick))}" x2="{RIGHT}" y2="{f(ypos(tick))}" class="g"/>')
    for i in label_runs(count):
        out.append(f'<line x1="{f(xpos(i))}" y1="{f(bottom)}" x2="{f(xpos(i))}" y2="{f(bottom + 4)}" class="g"/>')

    ends = []
    for position, label, points in series:
        segments, current, previous = [], [], None
        for i, value, _ in points:
            if previous is not None and i != previous + 1 and current:
                segments.append(current)
                current = []
            current.append((xpos(i), ypos(value)))
            previous = i
        segments.append(current)
        path = "".join(
            "M" + "L".join(f"{f(px)} {f(py)}" for px, py in seg) for seg in segments
        )
        if sum(len(seg) for seg in segments) > 1:
            out.append(f'<path d="{path}" class="l s{position}"/>')
        last = len(points) - 1
        for k, (i, value, _) in enumerate(points):
            radius = 4 if k == last else 3
            out.append(
                f'<circle cx="{f(xpos(i))}" cy="{f(ypos(value))}" r="{radius}" class="m f{position}"/>'
            )
        i, value, date = points[-1]
        ends.append((position, label, value, date, i == count - 1, ypos(value)))

    placed = spread([e[5] + 4 for e in ends], 15, top - 2, bottom + 8)
    for (position, label, value, date, current, ypx), label_y in zip(ends, placed):
        out.append(
            f'<path d="M{RIGHT + 10} {f(ypx)}L{RIGHT + 20} {f(label_y - 4)}h6" class="l s{position}"/>'
        )
        note = "" if current else f" ({history.day_time(date)})"
        out.append(
            f'<text x="{RIGHT + 30}" y="{f(label_y)}" font-size="12">{escape(label)} '
            f'<tspan class="d v">{escape(history.ns_text(value))}{escape(note)}</tspan></text>'
        )

    for tick in ticks_list:
        out.append(
            f'<text x="{LEFT - 15}" y="{f(ypos(tick) + 4)}" font-size="11.5" text-anchor="end" class="d v">'
            f"{escape(history.ns_text(tick))}</text>"
        )
    for i in label_runs(count):
        date = rows[i]["date"]
        out.append(
            f'<text x="{f(xpos(i))}" y="{f(bottom + 20)}" font-size="11.5" text-anchor="middle" class="d v">'
            f"{escape(history.day(date))}</text>"
        )
        out.append(
            f'<text x="{f(xpos(i))}" y="{f(bottom + 34)}" font-size="11" text-anchor="middle" class="d v">'
            f"{history.parse(date):%H:%M}</text>"
        )
    y = bottom + 56
    out.append(
        f'<text x="{f((LEFT + RIGHT) / 2)}" y="{f(y)}" font-size="12" text-anchor="middle" class="d">'
        f"{count} runs, times in UTC, oldest to newest, one step each however far apart</text>"
    )
    y += 24
    note = (
        "Only the runs of one machine are drawn: a number compares with its own machine's, "
        "and a run on another machine starts a history of its own."
    )
    for line in wrap(note, 125):
        out.append(f'<text x="16" y="{f(y)}" font-size="11.5" class="d">{escape(line)}</text>')
        y += 17
    height = math.ceil(y - 4)

    colors = palette["colors"]
    css = (
        f"text{{font-family:{FONT};fill:{palette['text']}}}.d{{fill:{palette['dim']}}}"
        ".b{font-weight:600}.v{font-variant-numeric:tabular-nums}"
        f".g{{stroke:{palette['grid']}}}"
        ".l{fill:none;stroke-width:2;stroke-linejoin:round;stroke-linecap:round}"
        f".m{{stroke:{palette['ring']};stroke-width:1.5}}"
        + "".join(f".s{k}{{stroke:{c}}}.f{k}{{fill:{c}}}" for k, c in enumerate(colors))
    )
    head = (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{height}" '
        f'viewBox="0 0 {WIDTH} {height}" role="img" aria-label="{escape(title)}">\n'
        f"<title>{escape(title)}</title>\n<style>{css}</style>\n"
    )
    return head + "\n".join(out) + "\n</svg>\n"


def label_runs(count):
    """Indices of the runs that get a tick label: all of them up to seven, else about six."""
    if count <= 7:
        return list(range(count))
    stride = math.ceil((count - 1) / 5)
    picked = list(range(0, count, stride))
    if picked[-1] != count - 1:
        if count - 1 - picked[-1] < stride / 2:
            picked.pop()
        picked.append(count - 1)
    return picked


def readme_block(drawn, skipped):
    parts = []
    for machine, rows in drawn.items():
        name = history.slug(machine)
        series = series_for(rows)
        first_day, last_day = history.day(rows[0]["date"]), history.day(rows[-1]["date"])
        span = first_day if first_day == last_day else f"{first_day} to {last_day}"
        moves = []
        for _, label, points in series:
            start, end = points[0], points[-1]
            if len(points) == 1:
                moves.append(f"{label} {history.ns_text(end[1])} (one run)")
            else:
                moves.append(f"{label} {history.ns_text(start[1])} to {history.ns_text(end[1])}")
        alt = "Nanoseconds per point, one line per case, logarithmic scale"
        parts.append(
            "<picture>\n"
            f'  <source media="(prefers-color-scheme: dark)" srcset="bench/trend-{name}-dark.svg">\n'
            f'  <img src="bench/trend-{name}-light.svg" alt="{escape(alt)}" width="100%">\n'
            "</picture>\n\n"
            f"Time per point: {len(rows)} runs from {span}, the first to the latest: "
            + "; ".join(moves)
            + "."
        )
    if skipped:
        parts.append(
            f"Not drawn until there are two runs on one machine: {len(skipped)} other machine(s) with one run."
        )
    return "\n\n".join(parts)


def main():
    machines = history.by_machine(history.load())
    drawn = {m: rows for m, rows in machines.items() if len(rows) >= 2}
    skipped = [m for m, rows in machines.items() if len(rows) < 2]
    out_dir = history.ROOT / "bench"
    for machine, rows in drawn.items():
        for theme in THEMES:
            path = out_dir / f"trend-{history.slug(machine)}-{theme}.svg"
            path.write_text(render(machine, rows, theme), encoding="utf-8")
    history.write_block("bench-trend", readme_block(drawn, skipped))


if __name__ == "__main__":
    main()
