#!/usr/bin/env python3
"""Draw the README bench cards (bench/cards/*.html and *.png) from the measurement files.

Speed comes from the same history line as the "How fast" table (how_fast.pick), size and memory
from bench/footprint.json, so a card shows no number the tables do not. The templates in
tools/bench/cards/ come from the benchmark-diagram skill; the PNG is a 1920×1080 Chrome headless
screenshot (`CHROME_PATH`, or a Chrome found on the usual paths). Standard library only.
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from html import escape
from pathlib import Path

import history
import how_fast

TEMPLATES = Path(__file__).resolve().parent / "cards"
OUT = history.ROOT / "bench" / "cards"
FOOTPRINT = history.ROOT / "bench" / "footprint.json"
BILLION = history.ROOT / "bench" / "billion.json"
MIB = 1024 * 1024
SHORT = {"4326": "WGS 84", "2154": "Lambert-93", "32631": "UTM 31N", "3857": "Web Mercator", "4267": "NAD27"}
CHROMES = [
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
]


def width(value, high):
    return f"{max(value / high * 100, 2):.0f}%"


def times(ratio):
    return f"{ratio:.1f}×"


def fill(template, data, shown):
    """Fill `{{key}}`, and show exactly the `data-slot` names in `shown`."""
    html = (TEMPLATES / f"{template}.html").read_text(encoding="utf-8")
    css = (TEMPLATES / "tokens.css").read_text(encoding="utf-8")
    html = re.sub(r'<link rel="stylesheet" href="tokens.css">', lambda _: f"<style>\n{css}\n</style>", html)

    def slot(match):
        name = match.group(1)
        hidden = "" if name in shown or not re.fullmatch(r"(r|s|st)\d", name) else " hidden"
        return f'data-slot="{name}"{hidden}'

    html = re.sub(r'data-slot="([a-zA-Z0-9]+)"(?: hidden)?', slot, html)
    if template == "grouped-hbars":
        data = data | row_sizes(len(shown))
    missing = set()

    def value(match):
        key = match.group(1)
        if key not in data:
            missing.add(key)
            return ""
        return escape(str(data[key]))

    html = re.sub(r"\{\{([a-zA-Z0-9_]+)\}\}", value, html)
    if missing:
        raise SystemExit(f"cards.py: {template} has no value for {sorted(missing)}")
    return html


def row_sizes(count):
    """Bar, value and name sizes (px) for `count` rows, so that few rows fill the 1080 px card."""
    per_row = 640 / count
    bar = min(80.0, per_row * 0.24)
    return {
        "barH": f"{bar:.0f}",
        "valSize": f"{min(52.0, bar * 0.6 + 14):.0f}",
        "nameSize": f"{min(40.0, 16 + per_row * 0.06):.0f}",
    }


def blank_rows(data, first, prefix, keys):
    for n in range(first, 9):
        for key in keys:
            data[f"{prefix}{n}{key}"] = "0%" if key.endswith("W") else ""


def speed_card(row, call, title, subtitle):
    cases = [c for c in how_fast.rows_of(row) if c[1].startswith(f"`{call}`")]
    data = {
        "eyebrow": f"PROJ-RS {row['result'].get('version', '1')} · AGAINST PROJ 9.9.0",
        "title": title,
        "accentWord": f"{times(how_fast.geomean([c[4] for c in cases]))} faster",
        "subtitle": subtitle,
        "legendNew": "proj-rs",
        "legendRival": "PROJ 9.9.0",
        "footerLeft": f"ns per point · best of {row['result'].get('rounds', 1)} runs · one core of a 4-vCPU CI runner",
        "footerRight": f"bench/history.jsonl · {history.day(row['date'])} · commit {row['commit']}",
    }
    for n, (pair, _, theirs, ours, ratio) in enumerate(cases, 1):
        source, target = pair.split("_")
        high = max(theirs, ours)
        data |= {
            f"r{n}Name": f"{SHORT[source]} → {SHORT[target]}",
            f"r{n}Caption": f"EPSG:{source} → {target}",
            f"r{n}NewW": width(ours, high),
            f"r{n}RivalW": width(theirs, high),
            f"r{n}NewVal": how_fast.ns(ours),
            f"r{n}RivalVal": how_fast.ns(theirs),
            f"r{n}Speedup": times(ratio),
            f"r{n}SpeedWord": "faster",
        }
    blank_rows(data, len(cases) + 1, "r", ("Name", "Caption", "NewW", "RivalW", "NewVal", "RivalVal", "Speedup", "SpeedWord"))
    return fill("grouped-hbars", data, {f"r{n}" for n in range(1, len(cases) + 1)})


def crate_version():
    for line in (history.ROOT / "Cargo.toml").read_text(encoding="utf-8").splitlines():
        if line.startswith("version"):
            return line.split('"')[1]
    raise SystemExit("cards.py: no version in Cargo.toml")


def size_card(fp):
    installs = {i["id"]: i["stripped_bytes"] / MIB for i in fp["size"]["installs"]}
    ours, proj, full = installs["ours"], installs["proj"], installs["proj_full"]
    data = {
        "eyebrow": f"PROJ-RS {crate_version()} · SIZE ON DISK",
        "title": f"{ours:.1f}",
        "accentWord": "MiB",
        "titleRest": "",
        "subtitle": "",
        "hero": f"{ours:.1f}",
        "heroUnit": "MiB",
        "heroSentence": "for a program that converts EPSG:4326 to 2154, proj.db included.",
        "heroDetail": (
            f"No library to install beside the C library. PROJ 9.9.0 needs {proj:.1f} MiB built "
            f"minimal, {full:.1f} MiB with TIFF and curl. With the network feature, proj-rs is "
            f"{installs['ours_network']:.1f} MiB."
        ),
        "s1Label": "PROJ 9.9.0, TIFF and curl",
        "s1Value": f"{full:.1f} MiB",
        "s1Width": width(full, full),
        "s1Class": "rival",
        "s2Label": "PROJ 9.9.0, minimal",
        "s2Value": f"{proj:.1f} MiB",
        "s2Width": width(proj, full),
        "s2Class": "rival",
        "s3Label": "proj-rs",
        "s3Value": f"{ours:.1f} MiB",
        "s3Width": width(ours, full),
        "s3Class": "accent",
        "footerLeft": "Bars are to scale · lower is better · stripped, shared libraries counted",
        "footerRight": f"bench/footprint.json · {history.day(fp['date'])} · commit {fp['commit']}",
    }
    return fill("hero-vs-bars", data, {"s1", "s2", "s3"})


def memory_card(fp):
    scenarios = fp["memory"]["scenarios"]
    names = {
        "empty": ("Process start", "nothing created"),
        "create": ("Create a transform", "EPSG:4326 → 2154"),
        "points": ("1,000,000 points", "one at a time"),
        "array": ("1,000,000 points", "one array of 15.3 MiB"),
        "multi": ("Several candidates", "EPSG:4267 → 4326, 1,000,000 points"),
    }
    rss = [(s["id"], s["ours"]["peak_rss_kb"] / 1024, s["proj"]["peak_rss_kb"] / 1024) for s in scenarios]
    rss = [r for r in rss if r[0] in names]
    create = next(r for r in rss if r[0] == "create")
    data = {
        "eyebrow": f"PROJ-RS {crate_version()} · PEAK MEMORY",
        "title": "Less memory, in" if all(r[1] < r[2] for r in rss) else "Peak memory, in",
        "accentWord": "every scenario" if all(r[1] < r[2] for r in rss) else f"{len(rss)} scenarios",
        "subtitle": f"peak resident memory (VmHWM), lower is better · creating a transform: {times(create[2] / create[1])} less",
        "legendNew": "proj-rs",
        "legendRival": "PROJ 9.9.0",
        "footerLeft": f"MiB · median of {fp['memory']['runs']} runs · output checked against PROJ",
        "footerRight": f"bench/footprint.json · {history.day(fp['date'])} · commit {fp['commit']}",
    }
    for n, (key, ours, theirs) in enumerate(rss, 1):
        high = max(ours, theirs)
        data |= {
            f"r{n}Name": names[key][0],
            f"r{n}Caption": names[key][1],
            f"r{n}NewW": width(ours, high),
            f"r{n}RivalW": width(theirs, high),
            f"r{n}NewVal": f"{ours:.1f} MiB",
            f"r{n}RivalVal": f"{theirs:.1f} MiB",
            f"r{n}Speedup": times(theirs / ours),
            f"r{n}SpeedWord": "less",
        }
    blank_rows(data, len(rss) + 1, "r", ("Name", "Caption", "NewW", "RivalW", "NewVal", "RivalVal", "Speedup", "SpeedWord"))
    return fill("grouped-hbars", data, {f"r{n}" for n in range(1, len(rss) + 1)})


def seconds(value):
    return f"{value:.1f} s" if value < 100 else f"{value:.0f} s"


def billion_card(line):
    run = line["result"]
    calls = [
        ("One point at a time", "convert · PROJ proj_trans", run["convert_s"], run["libproj_convert_s"]),
        ("A slice per pass", "convert_array · PROJ proj_trans_generic", run["convert_array_s"],
         run["libproj_convert_array_s"]),
    ]
    best = min(run["convert_s"], run["convert_array_s"])
    data = {
        "eyebrow": "PROJ-RS · ONE BILLION POINTS",
        "title": "A billion points in",
        "accentWord": seconds(best),
        "subtitle": f"{run['pair']}, {run['points'] // run['chunk']:,} passes of {run['chunk']:,} points "
                    "· seconds for the whole billion, shorter is faster",
        "legendNew": "proj-rs",
        "legendRival": "PROJ 9.9.0",
        "footerLeft": "seconds · one run · one core of a 4-vCPU CI runner · output checked against PROJ",
        "footerRight": f"bench/billion.json · {history.day(line['date'])} · commit {line['commit']}",
    }
    for n, (name, caption, ours, theirs) in enumerate(calls, 1):
        high = max(ours, theirs)
        data |= {
            f"r{n}Name": name,
            f"r{n}Caption": caption,
            f"r{n}NewW": width(ours, high),
            f"r{n}RivalW": width(theirs, high),
            f"r{n}NewVal": seconds(ours),
            f"r{n}RivalVal": seconds(theirs),
            f"r{n}Speedup": times(theirs / ours),
            f"r{n}SpeedWord": "faster",
        }
    blank_rows(data, len(calls) + 1, "r", ("Name", "Caption", "NewW", "RivalW", "NewVal", "RivalVal", "Speedup", "SpeedWord"))
    return fill("grouped-hbars", data, {f"r{n}" for n in range(1, len(calls) + 1)})


def chrome():
    found = os.environ.get("CHROME_PATH") or next((c for c in CHROMES if Path(c).exists()), None)
    return found or shutil.which("google-chrome") or shutil.which("chromium")


def render(browser, html, png):
    with tempfile.TemporaryDirectory() as tmp:
        page = Path(tmp) / "card.html"
        page.write_text(html.replace("<html", '<html class="export"', 1), encoding="utf-8")
        subprocess.run(
            [browser, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--force-device-scale-factor=1",
             "--window-size=1920,1080", f"--screenshot={png}", page.as_uri()],
            check=True, capture_output=True,
        )


def main():
    row = how_fast.pick(history.load())
    fp = json.loads(FOOTPRINT.read_text(encoding="utf-8"))
    cards = {
        "speed-array": speed_card(row, "convert_array", "A slice of points,",
                                  "convert_array on 200,000 points · nanoseconds per point, shorter is faster"),
        "speed-point": speed_card(row, "convert", "One point at a time,",
                                  "convert, one call per point · nanoseconds per point, shorter is faster"),
        "size": size_card(fp),
        "memory": memory_card(fp),
    }
    if BILLION.exists():
        cards["billion"] = billion_card(json.loads(BILLION.read_text(encoding="utf-8")))
    OUT.mkdir(parents=True, exist_ok=True)
    browser = chrome()
    for name, html in cards.items():
        (OUT / f"{name}.html").write_text(html, encoding="utf-8")
        if browser:
            render(browser, html, OUT / f"{name}.png")
    history.write_block(
        "bench-compare",
        ('<img src="bench/cards/billion.png" alt="Seconds to convert a billion points, EPSG:4326 to '
           '2154, proj-rs next to PROJ 9.9.0" width="100%">\n\n' if "billion" in cards else "")
        + '<img src="bench/cards/speed-array.png" alt="Nanoseconds per point of convert_array, proj-rs next to '
        'PROJ 9.9.0, five conversions" width="100%">\n\n'
        '<img src="bench/cards/speed-point.png" alt="Nanoseconds per point of convert, one point at a time, '
        'proj-rs next to PROJ 9.9.0, five conversions" width="100%">',
    )
    if not browser:
        sys.exit("cards.py: no Chrome found (set CHROME_PATH); the HTML cards are written, the PNGs are stale")


if __name__ == "__main__":
    main()
