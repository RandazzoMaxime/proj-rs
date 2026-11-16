#!/usr/bin/env python3
"""Amorce fuzz/corpus/<cible>/ depuis les fichiers de test du dépôt (voir docs/FUZZ.md).

Stdlib seulement. Le corpus n'est pas versionné : ce script le recrée. Tailles bornées.
"""

import hashlib
import json
import random
import re
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CORPUS = Path(__file__).resolve().parent / "corpus"
MAX_TEXT = 2000
MAX_GRID = 120_000
EXTENSIONS = {".tif": 0, ".tiff": 0, ".gtx": 1, ".gsb": 2, ".ct2": 3, ".dat": 4}


def put(target, data):
    folder = CORPUS / target
    folder.mkdir(parents=True, exist_ok=True)
    name = hashlib.sha1(data).hexdigest()
    (folder / name).write_bytes(data)


def parity_cases():
    for path in sorted((ROOT / "tests/parity").glob("**/cases.jsonl")):
        for line in path.read_text(encoding="utf-8").splitlines():
            yield json.loads(line)


def proj_strings():
    found = set()
    for path in sorted((ROOT / "tests/gie").glob("*.gie")):
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.strip().startswith(("operation", "+proj", "+init")) or "+proj=" in line:
                text = re.sub(r"^\s*operation\s+", "", line).strip()
                if "+proj" in text or "+init" in text:
                    found.add(text)
    for case in parity_cases():
        for arg in case["args"]:
            if "+proj=" in arg:
                found.add(arg)
    for path in sorted((ROOT / "tests/cli_yaml").glob("*.yaml")):
        for text in re.findall(r"[+]proj=[^'\"\n]*", path.read_text(encoding="utf-8")):
            found.add(text.strip())
    return sorted(text for text in found if 0 < len(text) <= MAX_TEXT)


def export_texts():
    """Un WKT ou un PROJJSON complet par sortie `projinfo -o` de la référence."""
    wanted = {}
    for case in parity_cases():
        if case["tool"] == "projinfo" and "-o" in case["args"]:
            fmt = case["args"][case["args"].index("-o") + 1]
            if fmt != "PROJ" and "--single-line" not in case["args"]:
                wanted[case["id"]] = fmt
    texts = {}
    path = ROOT / "tests/parity/export/expected.jsonl"
    if path.exists():
        for line in path.read_text(encoding="utf-8").splitlines():
            row = json.loads(line)
            if row["id"] in wanted and row["rc"] == 0:
                texts.setdefault(wanted[row["id"]], []).append("\n".join(row["out"]))
    rng = random.Random(1)
    picked = []
    for fmt in sorted(texts):
        pool = [t for t in texts[fmt] if len(t) <= MAX_TEXT]
        picked += rng.sample(pool, min(60, len(pool)))
    return picked


def codes():
    found = set()
    for case in parity_cases():
        for arg in case["args"]:
            if re.fullmatch(r"(EPSG|IGNF|ESRI):\w+", arg):
                found.add(arg)
    return sorted(found)


def transform_seeds():
    rng = random.Random(2)
    for _ in range(60):
        body = bytes([rng.randrange(3)]) + bytes(rng.randrange(256) for _ in range(40))
        put("transform", body)
    point = bytes([16]) + struct.pack("<d", 3.0)
    head = bytes([2]) + point * 8
    for pair in (("EPSG:4326", "EPSG:32631"), ("EPSG:4326", "EPSG:2154"),
                 ("+proj=longlat +ellps=GRS80", "+proj=utm +zone=31 +ellps=GRS80")):
        put("transform", head + pair[0].encode() + b"\0" + pair[1].encode())


def main():
    if CORPUS.exists() and any(CORPUS.iterdir()):
        print(f"{CORPUS} existe déjà : le supprimer pour repartir de zéro", file=sys.stderr)
    for text in proj_strings():
        put("proj_string", text.encode())
    for text in export_texts() + codes():
        put("user_input", text.encode())
    for text in proj_strings()[:300]:
        put("user_input", text.encode())
    for path in sorted(p for p in (ROOT / "data/tests").iterdir() if p.is_file()):
        data = path.read_bytes()
        if path.suffix in EXTENSIONS and len(data) <= MAX_GRID:
            put("grid_reader", bytes([EXTENSIONS[path.suffix]]) + data)
        elif path.suffix == ".json" and len(data) <= MAX_GRID:
            kind = 0 if "tinshift" in path.name else 1
            put("model_files", bytes([kind]) + data)
    transform_seeds()
    for folder in sorted(CORPUS.iterdir()):
        files = list(folder.iterdir())
        print(f"{folder.name}: {len(files)} fichiers, {sum(f.stat().st_size for f in files)} octets")


if __name__ == "__main__":
    main()
