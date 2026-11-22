#!/usr/bin/env python3
"""Print one value of the `metadata` table of a proj.db: `meta.py <proj.db> <key>`.

`meta.py data/proj.db PROJ.VERSION` gives the PROJ release the embedded database comes from.
Standard library only; the database is opened read-only.
"""
import sqlite3
import sys

if len(sys.argv) != 3:
    sys.exit("usage: meta.py <proj.db> <key>")
connection = sqlite3.connect(f"file:{sys.argv[1]}?mode=ro", uri=True)
row = connection.execute("SELECT value FROM metadata WHERE key = ?", (sys.argv[2],)).fetchone()
if row is None:
    sys.exit(f"no key {sys.argv[2]} in the metadata of {sys.argv[1]}")
print(row[0])
