"""Shared reading of bench/history.jsonl for the scripts of tools/bench."""
import json
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HISTORY = ROOT / "bench" / "history.jsonl"
README = ROOT / "README.md"


def load():
    """Every line of the history, oldest first, with `ns` (case name to ns per point)."""
    rows = []
    for line in HISTORY.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        row = json.loads(line)
        row["ns"] = {c["name"]: c["ns_per_point"] for c in row["result"]["cases"]}
        row["libproj_ns"] = {
            c["name"]: c["libproj_ns"] for c in row["result"].get("libproj", [])
        }
        rows.append(row)
    rows.sort(key=lambda row: row["date"])
    return rows


def parse(date):
    return datetime.strptime(date, "%Y-%m-%dT%H:%M:%SZ")


def day(date):
    when = parse(date)
    return f"{when:%b} {when.day}"


def day_time(date):
    return f"{day(date)} {parse(date):%H:%M}"


def replace_block(text, name, body):
    """Replace what sits between `<!-- name:start -->` and `<!-- name:end -->`."""
    start, end = f"<!-- {name}:start -->", f"<!-- {name}:end -->"
    head, found, rest = text.partition(start)
    if not found or end not in rest:
        raise SystemExit(f"README.md has no {start} ... {end} block")
    _, _, tail = rest.partition(end)
    return f"{head}{start}\n{body.rstrip()}\n{end}{tail}"


def write_block(name, body):
    text = README.read_text(encoding="utf-8")
    updated = replace_block(text, name, body)
    if updated != text:
        README.write_text(updated, encoding="utf-8")
