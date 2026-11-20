#!/usr/bin/env bash
# Regenerate data/proj.db from a PROJ release, the way the embedded database was made: build that
# release from its tag (tools/build-reference.sh), take the proj.db its build writes, and compare
# it with the one in the repository. The reference binaries of the release (cs2cs, projinfo, ...)
# are built with it, for the tests to compare against.
#
# Usage: tools/proj-db/update.sh <tag> [DIR]     (default DIR: target/proj-reference-<tag>)
#
# Writes data/proj.db and DIR/proj-db-diff.md (the difference, as Markdown). A database with the
# same content is not copied: the file would differ in bytes only. Exit status:
#   0  data/proj.db replaced, or already the same
#   3  the major layout version of the database changed: proj-rs reads the old layout, so the
#      database is not replaced; it needs a port of PROJ's reading code first (docs/PROJ-UPSTREAM.md)
#   other: the build failed.
# PROJ_REF_TIFF_CURL=ON builds the reference with TIFF and curl, as distributions build PROJ.
set -euo pipefail

tag=${1:?usage: tools/proj-db/update.sh <tag> [DIR]}
root=$(cd "$(dirname "$0")/../.." && pwd)
dir=${2:-$root/target/proj-reference-$tag}
meta() { python3 -I "$root/tools/proj-db/meta.py" "$1" "$2"; }

PROJ_REF_TAG=$tag "$root/tools/build-reference.sh" "$dir"
new=$dir/build/data/proj.db
old=$root/data/proj.db
[ -s "$new" ] || { echo "the build of PROJ $tag wrote no proj.db" >&2; exit 1; }

echo "PROJ.VERSION: $(meta "$old" PROJ.VERSION) -> $(meta "$new" PROJ.VERSION)"
echo "EPSG.VERSION: $(meta "$old" EPSG.VERSION) -> $(meta "$new" EPSG.VERSION)"
same=0
python3 -I "$root/tools/proj-db/diff.py" "$old" "$new" > "$dir/proj-db-diff.md" || same=$?
echo "difference: $dir/proj-db-diff.md"
if [ "$same" -eq 0 ]; then
  echo "data/proj.db already holds the database of PROJ $tag: nothing to replace"
  exit 0
fi
[ "$same" -eq 1 ] || { echo "diff.py failed" >&2; exit 1; }

old_major=$(meta "$old" DATABASE.LAYOUT.VERSION.MAJOR)
new_major=$(meta "$new" DATABASE.LAYOUT.VERSION.MAJOR)
if [ "$old_major" != "$new_major" ]; then
  echo "DATABASE.LAYOUT.VERSION.MAJOR $old_major -> $new_major: data/proj.db is not replaced" >&2
  exit 3
fi
cp "$new" "$old"
echo "data/proj.db is the database of PROJ $tag"
