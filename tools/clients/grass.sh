#!/usr/bin/env bash
# Run the pytest suite of GRASS GIS, a real client of libproj (g.proj, r.proj, v.proj, m.proj,
# the location and projection code of libgis), against PROJ 9.9.0 and against proj-rs
# (docs/CLIENTS-grass.md).
#
# Usage: tools/clients/grass.sh [WORKDIR]      (default: target/clients/grass)
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), a C and
# C++ compiler, flex, bison, GDAL with gdal-config (Debian: libgdal-dev), SQLite, python3 with
# venv, network access to github.com and PyPI. The source is pinned by its commit. GRASS is
# built once against PROJ 9.9.0. The second run swaps in the libproj of proj-rs through
# LD_LIBRARY_PATH (same soname), and puts the cs2cs of proj-rs on PATH, since m.proj runs it.
set -euo pipefail

TAG=8.5.0
COMMIT=8d6ceba290a2f0059a7301b9d4944455f313da72
root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/grass}
mkdir -p "$work"
work=$(cd "$work" && pwd)

[ -x "$ref/build/bin/cs2cs" ] || { echo "no reference PROJ in $ref: run tools/build-reference.sh" >&2; exit 1; }

rm -rf "$work/src"
git clone -q --depth 1 --branch "$TAG" https://github.com/OSGeo/grass.git "$work/src" 2>/dev/null
[ "$(git -C "$work/src" rev-parse HEAD)" = "$COMMIT" ] || { echo "$TAG is not $COMMIT" >&2; exit 1; }

prefix=$work/ref-prefix
rm -rf "$prefix" "$work/install"
cmake --install "$ref/build" --prefix "$prefix" > /dev/null
(cd "$work/src" && ./configure --prefix="$work/install" --without-x --without-cairo \
  --without-freetype --without-opengl --without-fftw --without-pdal --without-readline \
  --without-odbc --without-mysql --without-postgres --without-netcdf --without-liblas \
  --with-sqlite --with-gdal="$(command -v gdal-config)" --with-proj-includes="$prefix/include" \
  --with-proj-libs="$prefix/lib" --with-proj-share="$prefix/share/proj" \
  LDFLAGS="-Wl,--enable-new-dtags -Wl,-rpath,$prefix/lib") > "$work/configure.log" 2>&1
make -C "$work/src" -j"$(nproc)" > "$work/make.log" 2>&1
make -C "$work/src" install > "$work/install.log" 2>&1

"$root/capi/tests/dist.sh" "$work/ours" > /dev/null
cargo build -q --release --manifest-path "$root/Cargo.toml" --bin cs2cs
target=$(cargo metadata --format-version 1 --no-deps --manifest-path "$root/Cargo.toml" |
  python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
mkdir -p "$work/bin-ref" "$work/bin-ours"
ln -sf "$prefix/bin/cs2cs" "$work/bin-ref/cs2cs"
cp "$target/release/cs2cs" "$work/bin-ours/cs2cs"

venv=$work/venv
python3 -m venv --system-site-packages "$venv"
"$venv/bin/pip" install -q pytest==9.2.0 pytest-xdist==3.8.0 pytest-timeout==2.4.0 \
  psycopg2-binary==2.9.13

grass=$work/install/bin/grass
gisbase=$("$grass" --config path)
export PYTHONPATH=$("$grass" --config python_path) INITIAL_GISBASE=$gisbase INITIAL_PWD=$work/src
export PROJ_DATA=$prefix/share/proj PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES PROJ_NETWORK=OFF

# Tests marked needs_solo_run run alone, after the others.
run() {
  for part in par solo; do
    if [ "$part" = par ]; then select=(-m 'not needs_solo_run' -n 8); else select=(-m needs_solo_run); fi
    (cd "$work/src" && LD_LIBRARY_PATH="$gisbase/lib:$2" PATH="$3:$PATH" "$venv/bin/python" -m pytest \
      -p no:cacheprovider -k 'not testsuite' "${select[@]}" --timeout 600 -o junit_family=legacy \
      --junitxml="$work/junit-$1-$part.xml" -q > "$work/pytest-$1-$part.log" 2>&1 || true)
  done
  "$venv/bin/python" - "$work" "$1" > "$work/outcome-$1.txt" <<'EOF'
import sys
import xml.etree.ElementTree as ET
work, name = sys.argv[1:]
for part in ("par", "solo"):
    for case in ET.parse(f"{work}/junit-{name}-{part}.xml").iter("testcase"):
        state = next((c.tag for c in case if c.tag in ("failure", "error", "skipped")), "passed")
        print(f"{case.get('classname')}::{case.get('name')} {state}")
EOF
  sort -o "$work/outcome-$1.txt" "$work/outcome-$1.txt"
  echo "$1: $(cut -d' ' -f2 "$work/outcome-$1.txt" | sort | uniq -c | tr -s ' ' | paste -sd,)"
}
run ref "$prefix/lib" "$work/bin-ref"
run ours "$work/ours/lib" "$work/bin-ours"

if diff "$work/outcome-ref.txt" "$work/outcome-ours.txt"; then
  echo "every test has the same outcome with both libraries: $(wc -l < "$work/outcome-ref.txt")"
else
  echo "the outcomes differ" >&2
  exit 1
fi
