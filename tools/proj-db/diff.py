#!/usr/bin/env python3
"""Compare two proj.db files and write the difference as Markdown: `diff.py <old.db> <new.db>`.

The metadata (PROJ, EPSG, ESRI, IGNF and NKG versions, the layout of the database), then, table
by table, how many rows were added and removed. A row is compared whole: a changed row counts
once as removed and once as added. Schema changes (a table, view or index added, removed or
altered) are listed apart: they are what can break the queries of proj-rs. Standard library only.

Exit status, as `diff`: 0 when the two databases hold the same schema and rows, 1 otherwise.
"""
import sqlite3
import sys
from collections import Counter


def open_ro(path):
    return sqlite3.connect(f"file:{path}?mode=ro", uri=True)


def schema(connection):
    rows = connection.execute(
        "SELECT type, name, sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY name"
    )
    return {name: (kind, sql or "") for kind, name, sql in rows}


def rows(connection, table):
    return Counter(repr(row) for row in connection.execute(f'SELECT * FROM "{table}"'))


def main():
    if len(sys.argv) != 3:
        sys.exit("usage: diff.py <old proj.db> <new proj.db>")
    old, new = open_ro(sys.argv[1]), open_ro(sys.argv[2])
    old_meta = dict(old.execute("SELECT key, value FROM metadata"))
    new_meta = dict(new.execute("SELECT key, value FROM metadata"))
    old_schema, new_schema = schema(old), schema(new)

    out = ["### Métadonnées", "", "| Clé | Avant | Après |", "|---|---|---|"]
    for key in sorted(set(old_meta) | set(new_meta)):
        before, after = old_meta.get(key, "—"), new_meta.get(key, "—")
        mark = "" if before == after else " **(change)**"
        out.append(f"| `{key}` | {before} | {after}{mark} |")

    changed_schema = []
    for name in sorted(set(old_schema) | set(new_schema)):
        if name not in old_schema:
            changed_schema.append(f"- ajouté : {new_schema[name][0]} `{name}`")
        elif name not in new_schema:
            changed_schema.append(f"- retiré : {old_schema[name][0]} `{name}`")
        elif old_schema[name] != new_schema[name]:
            changed_schema.append(f"- modifié : {new_schema[name][0]} `{name}`")
    out += ["", "### Schéma", ""]
    out += changed_schema or ["Aucun changement de table, de vue ni d'index."]

    tables = sorted(
        name
        for name, (kind, _) in new_schema.items()
        if kind == "table" and name in old_schema and old_schema[name][0] == "table"
    )
    out += ["", "### Lignes, par table", "", "| Table | Avant | Après | Ajoutées | Retirées |",
            "|---|---:|---:|---:|---:|"]
    unchanged = 0
    differs = bool(changed_schema) or old_meta != new_meta
    for table in tables:
        before, after = rows(old, table), rows(new, table)
        added = sum((after - before).values())
        removed = sum((before - after).values())
        if added or removed:
            differs = True
            out.append(f"| `{table}` | {sum(before.values())} | {sum(after.values())} | {added} | {removed} |")
        else:
            unchanged += 1
    out.append("")
    out.append(f"{unchanged} table(s) sans changement.")
    if not differs:
        out = ["Aucune différence de contenu : mêmes métadonnées, même schéma, mêmes lignes."]
    print("\n".join(out))
    return 1 if differs else 0


if __name__ == "__main__":
    sys.exit(main())
