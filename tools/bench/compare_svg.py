#!/usr/bin/env python3
"""Draw bench/compare-{light,dark}.svg: proj-rs next to PROJ 9.9.0, case by case.

The bars come from the same history line as the README "How fast" table (how_fast.pick), so the
chart and the table cannot disagree. The README block between `<!-- bench-compare:start -->` and
`<!-- bench-compare:end -->` is rewritten with the pictures. Standard library only.
"""
import math
from html import escape
from textwrap import wrap

import history
import how_fast
from trend_svg import FONT, f, text_width

THEMES = {
    "light": {"text": "#1f2328", "dim": "#59636e", "grid": "#d1d9e0", "proj": "#8c959f", "ours": "#0969da"},
    "dark": {"text": "#f0f6fc", "dim": "#9198a1", "grid": "#262c36", "proj": "#6e7681", "ours": "#3987e5"},
}

WIDTH = 800
LABEL_RIGHT = 214
BAR_LEFT, BAR_RIGHT = 224, 650
BAR_H, BAR_GAP, GROUP_GAP = 11, 3, 15
CALLS = {"`convert`, one point at a time": "convert", "`convert_array`, 200,000 points": "convert_array"}


def axis_ticks(high):
    """Round linear ticks from 0 that reach past `high`."""
    raw = high / 5
    base = 10 ** math.floor(math.log10(raw))
    step = next(m * base for m in (1, 2, 2.5, 5, 10) if m * base >= raw)
    return [step * k for k in range(math.ceil(high / step) + 1)]


def ratio_text(ratio):
    if ratio >= 1.05:
        return f"{ratio:.1f}× faster"
    if ratio <= 1 / 1.05:
        return f"{1 / ratio:.1f}× slower"
    return f"≈ same ({ratio:.2f}×)"


def legend(out, y, items):
    """Swatches and names in one row at height `y`; `items` are (css class, name)."""
    x = 16.0
    for key, name in items:
        out.append(f'<rect x="{f(x)}" y="{y - 6}" width="14" height="10" rx="2" class="{key}"/>')
        out.append(f'<text x="{f(x + 20)}" y="{y + 3}" font-size="12.5">{escape(name)}</text>')
        x += 20 + text_width(name, 12.5) + 24


def document(title, out, bottom, palette, extra_css=""):
    """The SVG text around the drawn elements `out`, `bottom` being where the last one ends."""
    height = math.ceil(bottom)
    css = (
        f"text{{font-family:{FONT};fill:{palette['text']}}}.d{{fill:{palette['dim']}}}"
        ".b{font-weight:600}.v{font-variant-numeric:tabular-nums}"
        f".g{{stroke:{palette['grid']}}}.proj{{fill:{palette['proj']}}}.ours{{fill:{palette['ours']}}}"
        f"{extra_css}"
    )
    head = (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{height}" '
        f'viewBox="0 0 {WIDTH} {height}" role="img" aria-label="{escape(title)}">\n'
        f"<title>{escape(title)}</title>\n<style>{css}</style>\n"
    )
    return head + "\n".join(out) + "\n</svg>\n"


def render(row, theme):
    palette = THEMES[theme]
    cases = how_fast.rows_of(row)
    ticks = axis_ticks(max(max(c[2], c[3]) for c in cases))
    scale = (BAR_RIGHT - BAR_LEFT) / ticks[-1]
    title = f"proj-rs against PROJ 9.9.0, {row['machine']}"
    out = [f'<text x="16" y="26" font-size="17" class="b">{escape(title)}</text>']
    subtitle = (
        "Nanoseconds per point for the same points through both libraries, the best of "
        f"{row['result'].get('rounds', 1)} runs. Shorter is faster. One point at a time (convert) "
        "and a 200,000-point slice (convert_array)."
    )
    y = 45
    for line in wrap(subtitle, 112):
        out.append(f'<text x="16" y="{y}" font-size="13" class="d">{escape(line)}</text>')
        y += 19
    legend(out, y, (("proj", "PROJ 9.9.0 (libproj, C++)"), ("ours", "proj-rs (Rust)")))
    top = y + 22
    y = top
    rows = []
    for pair, calls, theirs, ours, ratio in cases:
        source, target = pair.split("_")
        rows.append((f"EPSG:{source} → {target}", CALLS[calls], theirs, ours, ratio, y))
        y += 2 * BAR_H + BAR_GAP + GROUP_GAP
    bottom = y - GROUP_GAP + 6
    for tick in ticks:
        tx = BAR_LEFT + tick * scale
        out.append(f'<line x1="{f(tx)}" y1="{top - 6}" x2="{f(tx)}" y2="{f(bottom)}" class="g"/>')
        out.append(
            f'<text x="{f(tx)}" y="{f(bottom + 16)}" font-size="11.5" text-anchor="middle" class="d v">'
            f"{escape(history.ns_text(tick) if tick else '0')}</text>"
        )
    for label, call, theirs, ours, ratio, gy in rows:
        mid = gy + BAR_H + BAR_GAP / 2
        out.append(f'<text x="{LABEL_RIGHT}" y="{f(mid - 2)}" font-size="12" text-anchor="end">{escape(label)}</text>')
        out.append(
            f'<text x="{LABEL_RIGHT}" y="{f(mid + 12)}" font-size="11.5" text-anchor="end" class="d">{call}</text>'
        )
        for key, value, by in (("proj", theirs, gy), ("ours", ours, gy + BAR_H + BAR_GAP)):
            width = max(value * scale, 1)
            out.append(
                f'<rect x="{BAR_LEFT}" y="{f(by)}" width="{f(width)}" height="{BAR_H}" rx="2" class="{key}"/>'
            )
            out.append(
                f'<text x="{f(BAR_LEFT + width + 5)}" y="{f(by + BAR_H - 1.5)}" font-size="11" class="d v">'
                f"{escape(history.ns_text(value))}</text>"
            )
        weight = ' class="b v"' if ratio >= 1.05 else ' class="d v"'
        out.append(
            f'<text x="{WIDTH - 16}" y="{f(mid + 4)}" font-size="12" text-anchor="end"{weight}>'
            f"{escape(ratio_text(ratio))}</text>"
        )
    y = bottom + 40
    note = (
        f"One line of bench/history.jsonl ({history.day_time(row['date'])} UTC, commit {row['commit']}), "
        "one machine: the ratios on another CPU, or against a libproj built with other flags, will differ."
    )
    for line in wrap(note, 125):
        out.append(f'<text x="16" y="{f(y)}" font-size="11.5" class="d">{escape(line)}</text>')
        y += 17
    return document(title, out, y - 4, palette)


def main():
    row = how_fast.pick(history.load())
    for theme in THEMES:
        path = history.ROOT / "bench" / f"compare-{theme}.svg"
        path.write_text(render(row, theme), encoding="utf-8")
    alt = f"Bar chart: nanoseconds per point, proj-rs next to PROJ 9.9.0, ten conversions, on {row['machine']}"
    history.write_block(
        "bench-compare",
        "<picture>\n"
        '  <source media="(prefers-color-scheme: dark)" srcset="bench/compare-dark.svg">\n'
        f'  <img src="bench/compare-light.svg" alt="{escape(alt)}" width="100%">\n'
        "</picture>",
    )


if __name__ == "__main__":
    main()
