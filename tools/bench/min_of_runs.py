#!/usr/bin/env python3
"""Merge several runs of proj-rs-bench into one record.

Reads one JSON record per line on stdin and prints one record on stdout. Each case keeps its
smallest ns per point over the runs, and each libproj entry keeps its smallest libproj time.
The ratio is then recomputed from the two minima, so it compares best with best.
"""
import json
import sys


def main() -> None:
    runs = [json.loads(line) for line in sys.stdin if line.strip()]
    if not runs:
        sys.exit("min_of_runs.py: no record on stdin")
    merged = runs[0]
    best = {}
    for run in runs:
        for case in run["cases"]:
            name = case["name"]
            best[name] = min(best.get(name, float("inf")), case["ns_per_point"])
    merged["cases"] = [
        {"name": case["name"], "ns_per_point": round(best[case["name"]], 6)}
        for case in runs[0]["cases"]
    ]
    libproj = {}
    for run in runs:
        for entry in run["libproj"]:
            name = entry["name"]
            libproj[name] = min(libproj.get(name, float("inf")), entry["libproj_ns"])
    merged["libproj"] = [
        {
            "name": entry["name"],
            "libproj_ratio": round(best[entry["name"]] / libproj[entry["name"]], 6),
            "libproj_ns": round(libproj[entry["name"]], 6),
        }
        for entry in runs[0]["libproj"]
    ]
    merged["rounds"] = len(runs)
    print(json.dumps(merged, separators=(",", ":")))


if __name__ == "__main__":
    main()
