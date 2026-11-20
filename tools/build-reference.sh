#!/usr/bin/env bash
# Build the reference PROJ 9.9.0 (cs2cs, projinfo, cct, gie) from the tagged
# sources, to compare proj-rs with the real thing on any pair or point, not
# only the cases written in tests/cli_yaml/*.yaml.
#
# Usage: tools/build-reference.sh [DIR]      (default: target/proj-reference)
# PROJ_REF_TAG=<tag> builds another release (tools/proj-db/update.sh uses it); default 9.9.0.
#
# Then, from the repository root:
#   DIR=target/proj-reference
#   PROJ_DATA=$DIR/build/data:$PWD/data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
#     $DIR/build/bin/cs2cs EPSG:4746 EPSG:4978 <<< "50.5 10 0"
#
# Needs git, cmake, a C++17 compiler and the SQLite3 development headers. The
# `sqlite3` program is optional: without it, a Python shim creates proj.db.
# TIFF and curl are off, so the reference reads the grids in their old formats
# (.gsb, .gtx, ...) found in data/tests, not .tif grids, and never downloads.
# PROJ_REF_TIFF_CURL=ON builds them in (needs libtiff and libcurl headers): bench/footprint.sh
# uses it to size a libproj that reads .tif grids and downloads, as distributions build it.
set -euo pipefail

TAG=${PROJ_REF_TAG:-9.9.0}
TIFF_CURL=${PROJ_REF_TIFF_CURL:-OFF}
DIR=${1:-target/proj-reference}
mkdir -p "$DIR"
DIR=$(cd "$DIR" && pwd)

if [ ! -d "$DIR/src/.git" ]; then
  git clone --quiet --depth 1 --branch "$TAG" https://github.com/OSGeo/PROJ.git "$DIR/src"
fi

sqlite_args=()
if ! command -v sqlite3 >/dev/null 2>&1; then
  mkdir -p "$DIR/shim"
  cat > "$DIR/shim/sqlite3" <<'EOF'
#!/usr/bin/env python3
# Stand-in for `sqlite3 DBFILE < script.sql`, the only form PROJ's build uses.
import sqlite3
import sys

args = [a for a in sys.argv[1:] if not a.startswith("-")]
conn = sqlite3.connect(args[0] if args else ":memory:")
conn.executescript(sys.stdin.read())
conn.commit()
conn.close()
EOF
  chmod +x "$DIR/shim/sqlite3"
  sqlite_args=(-DEXE_SQLITE3="$DIR/shim/sqlite3")
fi

cmake -S "$DIR/src" -B "$DIR/build" -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_TESTING=OFF -DENABLE_TIFF="$TIFF_CURL" -DENABLE_CURL="$TIFF_CURL" -DBUILD_PROJSYNC=OFF \
  "${sqlite_args[@]}" > "$DIR/cmake.log"
cmake --build "$DIR/build" --parallel "$(nproc 2>/dev/null || echo 4)" > "$DIR/build.log"

# projinfo without an argument prints its version line and exits 1.
{ "$DIR/build/bin/projinfo" 2>&1 || true; } | head -n 1
echo "reference binaries: $DIR/build/bin"
echo "PROJ_DATA=$DIR/build/data:$PWD/data/tests PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES"
