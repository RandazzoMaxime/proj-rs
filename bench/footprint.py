#!/usr/bin/env python3
"""Startup time and peak memory of this port's cs2cs against libproj's cs2cs.

    cargo build --release --bin cs2cs
    python3 bench/footprint.py [path/to/ours] [path/to/libproj/cs2cs]

Each run is a fresh process pinned to one core. The 1,000,000-point runs feed
random longitude/latitude pairs over France through stdin and discard stdout,
so they time the whole program, text parsing and printing included.
Linux only (uses ru_maxrss in KiB and taskset).
"""
import os, random, resource, statistics, subprocess, sys, tempfile, time

OURS = sys.argv[1] if len(sys.argv) > 1 else "target/release/cs2cs"
REF = sys.argv[2] if len(sys.argv) > 2 else "/usr/bin/cs2cs"

CASES = [
    ("1 point, EPSG:2154", 1, ["+proj=longlat", "+datum=WGS84", "+to", "EPSG:2154"], 5),
    ("1 point, UTM string", 1, ["+proj=longlat", "+ellps=WGS84", "+to", "+proj=utm", "+zone=31", "+ellps=WGS84"], 5),
    ("1,000,000 points, EPSG:2154", 1_000_000, ["+proj=longlat", "+datum=WGS84", "+to", "EPSG:2154"], 3),
    ("1,000,000 points, UTM string", 1_000_000, ["+proj=longlat", "+ellps=WGS84", "+to", "+proj=utm", "+zone=31", "+ellps=WGS84"], 3),
]

CHILD = (
    "import resource,subprocess,sys,time;"
    "t=time.perf_counter();"
    "p=subprocess.run(sys.argv[2:],stdin=open(sys.argv[1]),stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);"
    "print(time.perf_counter()-t,resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss,p.returncode)"
)

def once(exe, args, points_file):
    out = subprocess.check_output(["taskset", "-c", "1", sys.executable, "-c", CHILD, points_file, exe, "-d", "3", *args]).split()
    assert out[2] == b"0", "cs2cs failed"
    return float(out[0]) * 1000, int(out[1]) / 1024

random.seed(1)
with tempfile.TemporaryDirectory() as tmp:
    files = {}
    for n in {c[1] for c in CASES}:
        path = os.path.join(tmp, f"{n}.txt")
        with open(path, "w") as f:
            for _ in range(n):
                f.write(f"{random.uniform(-4, 8):.6f} {random.uniform(42, 51):.6f}\n" if n > 1 else "2.2945 48.8584\n")
        files[n] = path
    print(f"{'case':32s} {'tool':8s} {'wall ms':>9s} {'peak MB':>8s}")
    for name, n, args, repeats in CASES:
        for label, exe in (("libproj", REF), ("proj-rs", OURS)):
            runs = [once(exe, args, files[n]) for _ in range(repeats)]
            print(f"{name:32s} {label:8s} {statistics.median(r[0] for r in runs):9.1f} {statistics.median(r[1] for r in runs):8.1f}")
