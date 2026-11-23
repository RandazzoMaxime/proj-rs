#!/usr/bin/env python3
"""Split the triggers out of a proj.db, so that the read-only image embedded in proj-rs is smaller.

The triggers of PROJ's proj.db (38 of them, 193 KB of SQL) check the rows of an insert or an update;
reads never run them. SQLite parses every trigger when it loads the schema, which costs about 2.7 MiB
of heap in a process (docs/PORTING-memory.md). So the read-only image has no triggers, and
`data/proj-triggers.sql` holds them, in their original order, for the writable copies to replay.

Usage: split_triggers.py <full proj.db> <data dir>
       split_triggers.py --restore <data dir> <out.db>
The first writes <data dir>/proj.db (no triggers) and <data dir>/proj-triggers.sql. The second
writes the full database again, by replaying the triggers on the stripped one. Standard library only.
"""
import os
import shutil
import sqlite3
import sys
import tempfile


def restore(data_dir, out):
    shutil.copyfile(os.path.join(data_dir, "proj.db"), out)
    with open(os.path.join(data_dir, "proj-triggers.sql"), encoding="utf-8") as sql:
        conn = sqlite3.connect(out)
        conn.executescript(sql.read())
        conn.commit()
        conn.close()


def main(source, data_dir):
    with tempfile.TemporaryDirectory() as tmp:
        stripped = os.path.join(tmp, "proj.db")
        shutil.copyfile(source, stripped)
        conn = sqlite3.connect(stripped)
        triggers = conn.execute(
            "SELECT name, sql FROM sqlite_master WHERE type = 'trigger' ORDER BY rowid"
        ).fetchall()
        with open(os.path.join(data_dir, "proj-triggers.sql"), "w", encoding="utf-8", newline="\n") as out:
            for _, sql in triggers:
                out.write(sql + ";\n")
        for name, _ in triggers:
            conn.execute(f'DROP TRIGGER "{name}"')
        conn.commit()
        conn.execute("VACUUM")
        conn.close()
        shutil.copyfile(stripped, os.path.join(data_dir, "proj.db"))
    print(f"{len(triggers)} triggers -> proj-triggers.sql; proj.db without them")


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--restore":
        restore(sys.argv[2], sys.argv[3])
    elif len(sys.argv) == 3:
        main(sys.argv[1], sys.argv[2])
    else:
        sys.exit(__doc__)
