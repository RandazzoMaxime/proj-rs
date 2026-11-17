#!/usr/bin/env python3
"""Draw bench/{size,memory}-{light,dark}.svg from bench/footprint.json and rewrite the README block.

The block between `<!-- bench-footprint:start -->` and `<!-- bench-footprint:end -->` holds the two
pictures, a summary and the tables, all built from the file, so the README shows no number that
bench/footprint.sh did not measure. Themes, font and helpers come from compare_svg.py.
Standard library only.
"""
import json
from html import escape
from textwrap import wrap

import history
from compare_svg import BAR_LEFT, BAR_RIGHT, GROUP_GAP, LABEL_RIGHT, THEMES, WIDTH, axis_ticks, document, legend
from trend_svg import f

FOOTPRINT = history.ROOT / "bench" / "footprint.json"
MIB = 1024 * 1024
TONES = ".code{fill-opacity:1}.db{fill-opacity:.58}.libs{fill-opacity:.3}"
SIZE_H, SIZE_GAP = 20, 22

ROWS = {
    "proj": ("PROJ 9.9.0", "C program + libproj, TIFF and curl off"),
    "proj_full": ("PROJ 9.9.0", "C program + libproj, TIFF and curl on"),
    "capi": ("proj-rs as libproj", "same C program + libproj of capi/"),
    "ours": ("proj-rs", "Rust program, proj.db inside"),
    "ours_network": ("proj-rs, feature network", "Rust program, HTTPS grid download"),
}
SCENARIO_ROWS = {
    "empty": ("Start, nothing created", ""),
    "create": ("Create 4326 → 2154", ""),
    "points": ("Create + 1,000,000 points", "one at a time"),
    "array": ("Create + 1,000,000 points", "one array"),
    "multi": ("4267 → 4326, several candidates", "1,000,000 points, one at a time"),
}
VARIANT_NAMES = {
    "proj": "PROJ 9.9.0",
    "proj_full": "PROJ 9.9.0, TIFF and curl",
    "capi": "proj-rs C library",
    "ours": "proj-rs (Rust)",
}


def mib(value):
    return f"{value / MIB:.1f} MiB"


def mib_ticks_text(tick):
    return f"{tick:g}"


def load():
    return json.loads(FOOTPRINT.read_text(encoding="utf-8"))


def relative(ours, theirs, noun):
    """`ours` against `theirs`, as the README says it."""
    ratio = theirs / ours
    if ratio >= 1.05:
        return f"{ratio:.1f}× less {noun}"
    if ratio <= 1 / 1.05:
        return f"{1 / ratio:.1f}× more {noun}"
    return f"about the same {noun}"


def smaller_than(ours, theirs):
    ratio = theirs / ours
    if ratio >= 1.05:
        return f"{ratio:.1f}× smaller than"
    if ratio <= 1 / 1.05:
        return f"{1 / ratio:.1f}× larger than"
    return "about the size of"


def heading(out, title, subtitle):
    out.append(f'<text x="16" y="26" font-size="17" class="b">{escape(title)}</text>')
    y = 45
    for line in wrap(subtitle, 112):
        out.append(f'<text x="16" y="{y}" font-size="13" class="d">{escape(line)}</text>')
        y += 19
    return y


def footnote(out, y, data, what):
    note = (
        f"{what} Measured by bench/footprint.sh ({history.day_time(data['date'])} UTC, commit {data['commit']}), "
        "one machine, Linux x86-64: other systems, compilers or libc builds give other numbers."
    )
    for line in wrap(note, 125):
        out.append(f'<text x="16" y="{f(y)}" font-size="11.5" class="d">{escape(line)}</text>')
        y += 17
    return y


def render_size(data, theme):
    palette = THEMES[theme]
    installs = {i["id"]: i for i in data["size"]["installs"]}
    order = [key for key in ROWS if key in installs]
    scale_ticks = axis_ticks(max(installs[k]["stripped_bytes"] for k in order) / MIB)
    scale = (BAR_RIGHT - BAR_LEFT) / scale_ticks[-1]
    title = "Size on disk to convert EPSG:4326 to EPSG:2154"
    subtitle = (
        "Everything a program needs on disk to make this conversion: the program, the library, "
        "proj.db and the shared libraries they load (all but libc and libgcc_s), stripped. Shorter is smaller."
    )
    out = []
    y = heading(out, title, subtitle)
    items = (("proj code", "program and library"), ("proj db", "proj.db"), ("proj libs", "other shared libraries"))
    legend(out, y, items)
    top = y + 22
    y = top
    spans = []
    for key in order:
        spans.append((key, y))
        y += SIZE_H + SIZE_GAP
    bottom = y - SIZE_GAP + 6
    for tick in scale_ticks:
        tx = BAR_LEFT + tick * scale
        out.append(f'<line x1="{f(tx)}" y1="{top - 6}" x2="{f(tx)}" y2="{f(bottom)}" class="g"/>')
        out.append(
            f'<text x="{f(tx)}" y="{f(bottom + 16)}" font-size="11.5" text-anchor="middle" class="d v">'
            f"{mib_ticks_text(tick) if tick else '0'}</text>"
        )
    out.append(
        f'<text x="{BAR_RIGHT + 24}" y="{f(bottom + 16)}" font-size="11.5" class="d">MiB</text>'
    )
    for key, by in spans:
        install = installs[key]
        side = install["side"]
        first, second = ROWS[key]
        out.append(f'<text x="{LABEL_RIGHT}" y="{f(by + 9)}" font-size="12" text-anchor="end">{escape(first)}</text>')
        out.append(
            f'<text x="{LABEL_RIGHT}" y="{f(by + 22)}" font-size="11" text-anchor="end" class="d">{escape(second)}</text>'
        )
        x = BAR_LEFT
        for part in ("code", "db", "libs"):
            width = install["parts"][part]["stripped_bytes"] / MIB * scale
            if width <= 0:
                continue
            out.append(
                f'<rect x="{f(x)}" y="{f(by)}" width="{f(max(width - 1.5, 0.5))}" height="{SIZE_H}" '
                f'class="{side} {part}"/>'
            )
            x += width
        out.append(
            f'<text x="{f(x + 6)}" y="{f(by + SIZE_H - 6)}" font-size="12" class="v b">'
            f"{escape(mib(install['stripped_bytes']))}</text>"
        )
    y = footnote(out, bottom + 40, data, "Bytes on disk after strip.")
    return document(title, out, y - 4, palette, TONES)


def render_memory(data, theme):
    palette = THEMES[theme]
    scenarios = data["memory"]["scenarios"]
    ticks = axis_ticks(max(max(s["proj"]["peak_rss_kb"], s["ours"]["peak_rss_kb"]) for s in scenarios) / 1024)
    right = BAR_RIGHT - 60
    scale = (right - BAR_LEFT) / ticks[-1]
    title = "Peak memory of the same conversions, proj-rs against PROJ 9.9.0"
    subtitle = (
        f"Peak resident memory of a process that runs one scenario, the median of {data['memory']['runs']} runs, "
        "the same points on both sides. Shorter is smaller."
    )
    out = []
    y = heading(out, title, subtitle)
    legend(out, y, (("proj", "PROJ 9.9.0 (libproj, C++)"), ("ours", "proj-rs (Rust)")))
    top = y + 22
    y = top
    groups = []
    bar_h, bar_gap = 11, 3
    for scenario in scenarios:
        groups.append((scenario, y))
        y += 2 * bar_h + bar_gap + GROUP_GAP
    bottom = y - GROUP_GAP + 6
    for tick in ticks:
        tx = BAR_LEFT + tick * scale
        out.append(f'<line x1="{f(tx)}" y1="{top - 6}" x2="{f(tx)}" y2="{f(bottom)}" class="g"/>')
        out.append(
            f'<text x="{f(tx)}" y="{f(bottom + 16)}" font-size="11.5" text-anchor="middle" class="d v">'
            f"{mib_ticks_text(tick) if tick else '0'}</text>"
        )
    out.append(f'<text x="{right + 24}" y="{f(bottom + 16)}" font-size="11.5" class="d">MiB</text>')
    for scenario, gy in groups:
        mid = gy + bar_h + bar_gap / 2
        first, second = SCENARIO_ROWS.get(scenario["id"], (scenario["label"], ""))
        out.append(f'<text x="{LABEL_RIGHT}" y="{f(mid - 2)}" font-size="12" text-anchor="end">{escape(first)}</text>')
        if second:
            out.append(
                f'<text x="{LABEL_RIGHT}" y="{f(mid + 12)}" font-size="11.5" text-anchor="end" class="d">'
                f"{escape(second)}</text>"
            )
        for key, by in (("proj", gy), ("ours", gy + bar_h + bar_gap)):
            value = scenario[key]["peak_rss_kb"] / 1024
            width = max(value * scale, 1)
            out.append(f'<rect x="{BAR_LEFT}" y="{f(by)}" width="{f(width)}" height="{bar_h}" rx="2" class="{key}"/>')
            out.append(
                f'<text x="{f(BAR_LEFT + width + 5)}" y="{f(by + bar_h - 1.5)}" font-size="11" class="d v">'
                f"{value:.1f} MiB</text>"
            )
        text = relative(scenario["ours"]["peak_rss_kb"], scenario["proj"]["peak_rss_kb"], "memory")
        out.append(
            f'<text x="{WIDTH - 16}" y="{f(mid + 4)}" font-size="12" text-anchor="end" class="d v">{escape(text)}</text>'
        )
    y = footnote(
        out, bottom + 40, data,
        f"VmHWM of the process: the loader, the mapped libraries and the {data['memory']['points'] * 16 / MIB:.1f} MiB "
        "buffer of the array scenario count.",
    )
    return document(title, out, y - 4, palette)


def named_bytes(data, install, prefixes):
    sizes = {f["name"]: f["stripped_bytes"] for f in data["size"]["files"]}
    names = [n for n in install["parts"]["libs"]["items"] if n.startswith(prefixes)]
    return sum(sizes[n] for n in names) if names else None


def size_table(data):
    lines = [
        "| Install | Program and library | proj.db | Other shared libraries | of which OpenSSL | "
        "of which libcurl | Total, stripped | Total, as built |",
        "|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for install in data["size"]["installs"]:
        first, second = ROWS[install["id"]]
        parts = install["parts"]
        ssl = named_bytes(data, install, ("libssl", "libcrypto"))
        curl = named_bytes(data, install, ("libcurl",))
        lines.append(
            f"| {first}, {second} | {mib(parts['code']['stripped_bytes'])} | {mib(parts['db']['stripped_bytes'])} "
            f"| {mib(parts['libs']['stripped_bytes'])} | {'none' if ssl is None else mib(ssl)} "
            f"| {'none' if curl is None else mib(curl)} | **{mib(install['stripped_bytes'])}** "
            f"| {mib(install['bytes'])} |"
        )
    return "\n".join(lines)


def libs_text(data):
    lines = []
    for install in data["size"]["installs"]:
        libs = install["parts"]["libs"]["items"]
        names = ", ".join(f"`{n}`" for n in libs) if libs else "none"
        lines.append(f"- {ROWS[install['id']][0]}, {ROWS[install['id']][1]}: {names}.")
    return "\n".join(lines)


def memory_table(data, field, scale, unit, title):
    key_cols = [v for v in VARIANT_NAMES if all(v in s for s in data["memory"]["scenarios"])]
    lines = [
        f"| {title} | " + " | ".join(VARIANT_NAMES[v] for v in key_cols) + " |",
        "|---|" + "---:|" * len(key_cols),
    ]
    for s in data["memory"]["scenarios"]:
        cells = []
        for v in key_cols:
            value = s[v].get(field)
            cells.append("n/a" if value is None else f"{value / scale:.1f} {unit}")
        lines.append(f"| {s['label']} | " + " | ".join(cells) + " |")
    return "\n".join(lines)


def block(data):
    installs = {i["id"]: i for i in data["size"]["installs"]}
    ours, proj, full = installs["ours"], installs["proj"], installs["proj_full"]
    scenarios = {s["id"]: s for s in data["memory"]["scenarios"]}
    empty_rust = data["size"]["empty_rust_program"]
    program = next(f for f in data["size"]["files"] if f["name"] == "footprint-rs")
    db = data["size"]["embedded_db_bytes"]
    created, points, array, multi = (scenarios[k] for k in ("create", "points", "array", "multi"))
    rss = lambda s, v: s[v]["peak_rss_kb"]
    network_text = ""
    if "ours_network" in installs:
        net = installs["ours_network"]
        ssl = named_bytes(data, net, ("libssl", "libcrypto"))
        network_text = (
            f"With the `network` feature (the HTTPS grid download, TLS from the system) the program is "
            f"{mib(net['stripped_bytes'])}"
            + (f", of which {mib(ssl)} is OpenSSL" if ssl else "")
            + f": {smaller_than(net['stripped_bytes'], full['stripped_bytes'])} the PROJ with curl. "
        )
    lines = [
        "The same conversion, EPSG:4326 to EPSG:2154, measured on the same machine "
        "(Linux x86-64, glibc), proj-rs against PROJ 9.9.0.",
        "",
        "<picture>",
        '  <source media="(prefers-color-scheme: dark)" srcset="bench/size-dark.svg">',
        '  <img src="bench/size-light.svg" alt="Stacked bar chart: size on disk of the install needed '
        'to convert EPSG:4326 to EPSG:2154, PROJ 9.9.0 and proj-rs" width="100%">',
        "</picture>",
        "",
        "<picture>",
        '  <source media="(prefers-color-scheme: dark)" srcset="bench/memory-dark.svg">',
        '  <img src="bench/memory-light.svg" alt="Bar chart: peak memory of five scenarios, proj-rs next to '
        'PROJ 9.9.0" width="100%">',
        "</picture>",
        "",
        f"On disk, a Rust program with proj-rs is {mib(ours['stripped_bytes'])} with `proj.db` included "
        f"({mib(db)} of it) and no library other than the C library: "
        f"{smaller_than(ours['stripped_bytes'], proj['stripped_bytes'])} a C program with PROJ 9.9.0 built "
        f"as in the speed comparison ({mib(proj['stripped_bytes'])}), and "
        f"{smaller_than(ours['stripped_bytes'], full['stripped_bytes'])} one with a PROJ built with TIFF "
        f"and curl ({mib(full['stripped_bytes'])}). {network_text}"
        f"The code of proj-rs alone, `proj.db` left out, "
        f"is {mib(ours['parts']['code']['stripped_bytes'])}; an empty Rust program is "
        f"{mib(empty_rust['stripped_bytes'])}.",
        "",
        f"In memory, creating the transform peaks at {rss(created, 'ours') / 1024:.1f} MiB for proj-rs and "
        f"{rss(created, 'proj') / 1024:.1f} MiB for PROJ "
        f"({relative(rss(created, 'ours'), rss(created, 'proj'), 'memory')}). With a million points one at a time "
        f"it is {rss(points, 'ours') / 1024:.1f} MiB and {rss(points, 'proj') / 1024:.1f} MiB; as one array, "
        f"{rss(array, 'ours') / 1024:.1f} MiB and {rss(array, 'proj') / 1024:.1f} MiB, "
        f"the array of points itself being {data['memory']['points'] * 16 / MIB:.1f} MiB; with several candidate operations (EPSG:4267 to 4326), "
        f"{rss(multi, 'ours') / 1024:.1f} MiB and {rss(multi, 'proj') / 1024:.1f} MiB.",
        "",
        "<details>",
        "<summary><b>Size and memory, component by component</b></summary>",
        "",
        "<br>",
        "",
        "Size on disk, after `strip --strip-unneeded`. A program is counted with every shared "
        f"library `ldd` lists for it, except the C library and the unwinder (libc, libm, libdl, libpthread, "
        f"librt, libutil, libgcc_s), on both sides. {history.day_time(data['date'])} UTC, commit "
        f"`{data['commit']}`.",
        "",
        size_table(data),
        "",
        "OpenSSL is `libssl` plus `libcrypto`, already inside the third column. proj-rs links it only "
        "with the `network` feature, for its HTTPS grid download, and takes it from the system "
        "(`native-tls`); PROJ pulls it through libcurl, when built with curl, together with the "
        "other libraries of curl (`libnghttp2`, `libssh`, `libgssapi_krb5`, `libldap`, `libgnutls` and more), "
        "which count in the third column but not in the libcurl column.",
        "",
        "Shared libraries counted in the third column:",
        "",
        libs_text(data),
        "",
        f"The Rust program is {mib(program['stripped_bytes'])} stripped, of which {mib(db)} is the "
        f"embedded `proj.db`; the empty Rust program of the same toolchain is "
        f"{mib(empty_rust['stripped_bytes'])}. The code of proj-rs thus adds "
        f"{mib(ours['parts']['code']['stripped_bytes'] - empty_rust['stripped_bytes'])}, SQLite and the "
        "TIFF reader included.",
        "",
        memory_table(data, "peak_rss_kb", 1024, "MiB", "Peak resident memory"),
        "",
        memory_table(data, "peak_heap_bytes", MIB, "MiB", "Peak heap (valgrind massif)"),
        "",
        f"Median of {data['memory']['runs']} runs for the resident memory (`VmHWM`, read by the program "
        "itself); the heap is one massif run. Each process runs one scenario on the same "
        f"{data['memory']['points']:,} points, and its output is checked against PROJ's. Every "
        "process starts with its own loader and libraries, so the first row is the floor. "
        "What exactly is measured, and how to rerun it: [docs/BENCHMARK.md](docs/BENCHMARK.md).",
        "",
        "</details>",
    ]
    return "\n".join(lines)


def main():
    data = load()
    for theme in THEMES:
        (history.ROOT / "bench" / f"size-{theme}.svg").write_text(render_size(data, theme), encoding="utf-8")
        (history.ROOT / "bench" / f"memory-{theme}.svg").write_text(render_memory(data, theme), encoding="utf-8")
    history.write_block("bench-footprint", block(data))


if __name__ == "__main__":
    main()
