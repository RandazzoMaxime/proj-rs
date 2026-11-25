#!/usr/bin/env bash
# Run the pytest suite of GRASS GIS, a real client of libproj (g.proj, r.proj, v.proj, m.proj,
# the location and projection code of libgis), against PROJ 9.9.0 and against proj-rs
# (docs/CLIENTS-grass.md).
#
# Usage: tools/clients/grass.sh [WORKDIR]      (default: target/clients/grass)
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), a C and
# C++ compiler, python3 with venv, network access to github.com (the source, and the add-ons
# g.extension installs in its tests) and PyPI. On Debian or Ubuntu run as root, the script
# installs the system packages it builds against (flex, bison, GDAL, SQLite, FFTW, ...). The
# source is pinned by its commit. GRASS is built once against PROJ 9.9.0. The second run swaps in
# the libproj of proj-rs through LD_LIBRARY_PATH (same soname), and puts the cs2cs of proj-rs on
# PATH, since m.proj runs it.
#
# The environment is the one the suite expects, so that a failure says something about libproj:
# FFTW for r.surf.fractal (and the many tests whose data it generates), the Python packages of
# the venv for the modules too (python3 of the venv first on PATH: psycopg2 for the temporal
# framework, IPython and ipywidgets for grass.jupyter), and, when the script runs as root, the
# suite runs as an unprivileged user: r.pack checks that it cannot write into a read-only
# directory, which root always can. Which libproj each pass loads is proven by
# libproj_proof.sh (ldd, no DT_RPATH, LD_DEBUG=libs, the proj_count.c interposer for the whole
# pass).
set -euo pipefail

TAG=8.5.0
COMMIT=8d6ceba290a2f0059a7301b9d4944455f313da72
BENCH_USER=grass-bench
# Processes of a pass that call one of the functions proj_count.c counts (g.proj and cs2cs, about
# 120 of them; some 5400 more map libproj without calling those).
MIN_CALLERS=100
root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/grass}
mkdir -p "$work"
work=$(cd "$work" && pwd)
. "$root/tools/clients/libproj_proof.sh"

[ -x "$ref/build/bin/cs2cs" ] || { echo "no reference PROJ in $ref: run tools/build-reference.sh" >&2; exit 1; }

packages=(flex bison libgdal-dev gdal-bin libsqlite3-dev libfftw3-dev libpng-dev libbz2-dev
  libzstd-dev zlib1g-dev libtiff-dev python3-venv)
if [ "$(id -u)" = 0 ] && command -v apt-get > /dev/null; then
  missing=()
  for p in "${packages[@]}"; do dpkg -s "$p" > /dev/null 2>&1 || missing+=("$p"); done
  [ ${#missing[@]} = 0 ] || { apt-get update -q > /dev/null; apt-get install -y -q --no-install-recommends "${missing[@]}" > /dev/null; }
fi
for tool in gdal-config flex bison cc c++ ldd readelf setpriv; do
  command -v "$tool" > /dev/null || { echo "need $tool on the PATH" >&2; exit 1; }
done

rm -rf "$work/src"
git clone -q --depth 1 --branch "$TAG" https://github.com/OSGeo/grass.git "$work/src" 2>/dev/null
[ "$(git -C "$work/src" rev-parse HEAD)" = "$COMMIT" ] || { echo "$TAG is not $COMMIT" >&2; exit 1; }

prefix=$work/ref-prefix
rm -rf "$prefix" "$work/install"
cmake --install "$ref/build" --prefix "$prefix" > /dev/null
(cd "$work/src" && ./configure --prefix="$work/install" --without-x --without-cairo \
  --without-freetype --without-opengl --with-fftw --without-pdal --without-readline \
  --without-odbc --without-mysql --without-postgres --without-netcdf --without-liblas \
  --with-sqlite --with-gdal="$(command -v gdal-config)" --with-proj-includes="$prefix/include" \
  --with-proj-libs="$prefix/lib" --with-proj-share="$prefix/share/proj" \
  LDFLAGS="-Wl,--enable-new-dtags -Wl,-rpath,$prefix/lib") > "$work/configure.log" 2>&1
grep -q 'FFTW support: *yes' "$work/configure.log" || { echo "configure did not find FFTW" >&2; exit 1; }
make -C "$work/src" -j"$(nproc)" > "$work/make.log" 2>&1
make -C "$work/src" install > "$work/install.log" 2>&1

"$root/capi/tests/dist.sh" "$work/ours" > /dev/null
cargo build -q --release --manifest-path "$root/Cargo.toml" --bin cs2cs
target=$(cargo metadata --format-version 1 --no-deps --manifest-path "$root/Cargo.toml" |
  python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
rm -rf "$work/bin-ref" "$work/bin-ours"
mkdir -p "$work/bin-ref" "$work/bin-ours"
ln -sf "$prefix/bin/cs2cs" "$work/bin-ref/cs2cs"
cp "$target/release/cs2cs" "$work/bin-ours/cs2cs"
proof_build "$work" "$prefix/include"
proof_distinct "$prefix/lib" "$work/ours/lib"

venv=$work/venv
rm -rf "$venv"
python3 -m venv --system-site-packages "$venv"
"$venv/bin/pip" install -q pytest==9.2.0 pytest-xdist==3.8.0 pytest-timeout==2.4.0 \
  psycopg2-binary==2.9.13 ipython==9.19.1 ipywidgets==8.1.9

grass=$work/install/bin/grass
gisbase=$("$grass" --config path)
export PYTHONPATH=$("$grass" --config python_path) INITIAL_GISBASE=$gisbase INITIAL_PWD=$work/src
export PROJ_DATA=$prefix/share/proj PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES PROJ_NETWORK=OFF
export PYTHONDONTWRITEBYTECODE=1

# As root, the suite runs as an unprivileged user that owns the tree (r.pack, see the header).
# The CA bundles the environment points to may sit in a directory that user cannot read.
as_user=()
if [ "$(id -u)" = 0 ]; then
  id "$BENCH_USER" > /dev/null 2>&1 \
    || useradd --system --user-group --no-create-home --shell /usr/sbin/nologin "$BENCH_USER"
  as_user=(setpriv --reuid="$BENCH_USER" --regid="$BENCH_USER" --clear-groups)
  for var in SSL_CERT_FILE GIT_SSL_CAINFO CURL_CA_BUNDLE REQUESTS_CA_BUNDLE PIP_CERT; do
    file=${!var:-}
    [ -n "$file" ] && [ -f "$file" ] || continue
    if ! "${as_user[@]}" test -r "$file"; then
      cp "$file" "$work/ca-bundle.crt"
      chmod 644 "$work/ca-bundle.crt"
      export "$var=$work/ca-bundle.crt"
    fi
  done
fi

# Tests marked needs_solo_run run alone, after the others. Each pass starts from an empty home
# (g.extension installs its add-ons there).
run() {
  local name=$1 libdir=$2 bin=$3 path="$gisbase/lib:$2" out=$work/pass-$1 log=$work/projcount-$1.log
  rm -rf "$out" "$work/home" "$log"
  mkdir -p "$out" "$work/home"
  (cd "$work" && proof_static "$name" "$libdir" "$path" "$gisbase/bin/g.proj" "$gisbase/bin/r.proj" \
    "$gisbase/lib/libgrass_gproj.so")
  (cd "$work/home" && HOME=$work/home proof_call "$name" "$libdir" "$path" \
    "$grass" --tmp-project EPSG:3358 --exec g.proj -jf)
  touch "$log"
  [ ${#as_user[@]} = 0 ] || chown -R "$BENCH_USER:$BENCH_USER" "$work/src" "$work/home" "$out" "$log"
  for part in par solo; do
    if [ "$part" = par ]; then select=(-m 'not needs_solo_run' -n 8); else select=(-m needs_solo_run); fi
    (cd "$work/src" && "${as_user[@]}" env HOME="$work/home" $(proof_env "$path" "$log") \
      PATH="$bin:$venv/bin:$PATH" "$venv/bin/python" -m pytest \
      -p no:cacheprovider -k 'not testsuite' "${select[@]}" --timeout 600 -o junit_family=legacy \
      --junitxml="$out/junit-$part.xml" -q -rfEs > "$out/pytest-$part.log" 2>&1 || true)
  done
  proof_log "$name" "$libdir" "$log" "$MIN_CALLERS"
  "$venv/bin/python" - "$out" > "$work/outcome-$name.txt" <<'EOF'
import sys
import xml.etree.ElementTree as ET
out = sys.argv[1]
for part in ("par", "solo"):
    for case in ET.parse(f"{out}/junit-{part}.xml").iter("testcase"):
        state = next((c.tag for c in case if c.tag in ("failure", "error", "skipped")), "passed")
        print(f"{case.get('classname')}::{case.get('name')} {state}")
EOF
  sort -o "$work/outcome-$name.txt" "$work/outcome-$name.txt"
  echo "$name: $(awk '{ print $NF }' "$work/outcome-$name.txt" | sort | uniq -c | tr -s ' ' | paste -sd,)"
  grep -hE '^(FAILED|ERROR|SKIPPED)' "$out"/pytest-*.log | sed "s/^/$name: /" || true
}
echo "GRASS $TAG ($COMMIT), GDAL $(gdal-config --version), $("$venv/bin/python" --version)," \
  "pytest $("$venv/bin/python" -c 'import pytest; print(pytest.__version__)'), run as $("${as_user[@]}" id -un)"
run ref "$prefix/lib" "$work/bin-ref"
run ours "$work/ours/lib" "$work/bin-ours"

if diff "$work/outcome-ref.txt" "$work/outcome-ours.txt"; then
  echo "every test has the same outcome with both libraries: $(wc -l < "$work/outcome-ref.txt")"
else
  echo "the outcomes differ" >&2
  exit 1
fi
