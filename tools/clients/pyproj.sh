#!/usr/bin/env bash
# Run the test suite of pyproj, a real client of libproj, against PROJ 9.9.0 and against the C
# library of proj-rs, and list what fails with proj-rs only (docs/CLIENTS-pyproj.md).
#
# Usage: tools/clients/pyproj.sh [WORKDIR]          (default: target/clients/pyproj)
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference; its
# source is reused), cmake, python3 with venv and its headers, a C and C++ compiler, the libtiff,
# libcurl and SQLite3 development headers (installed by the script when it runs as root on Debian
# or Ubuntu), network access to PyPI and, for the tests marked `network`, to cdn.proj.org
# (through HTTPS_PROXY when it is set). The pyproj source is pinned by its sha256 and the Python
# packages by their versions. pyproj is built twice, once against each libproj; both runs read the
# same data directory, the share/proj of the PROJ 9.9.0 install, so a difference comes from the
# library.
#
# The environment is the one the suite expects, so that a failure says something about libproj:
# - both libraries read GeoTIFF grids and download, as distributions and the pyproj wheels build
#   PROJ: the reference is PROJ 9.9.0 built with TIFF and curl (PROJ_REF_TIFF_CURL=ON of
#   tools/build-reference.sh, in PROJ_REFERENCE_TIFF_CURL, default
#   target/proj-reference-tiff-curl), and the C library of proj-rs is built with its `network`
#   feature. Without TIFF, the reference cannot read the grid test_transformer_group__download_grids
#   downloads;
# - every optional test dependency of pyproj (shapely, pandas, xarray), the `pyproj` command of the
#   venv on PATH (test_cli runs it), and the whole suite, the tests marked `network` included.
# Which libproj each run loads is proven by libproj_proof.sh (ldd of the extension modules, no
# DT_RPATH, LD_DEBUG=libs on a real transformation, the proj_count.c interposer for the whole
# pytest run). The script fails when a test has a different outcome in the two runs.
set -euo pipefail

PYPROJ_VERSION=3.8.0
PYPROJ_SHA256=efa59725bba68bf97fa808b61302df32934acdceb6a5c92a8dd0e71dc266a876
PACKAGES=(pytest==9.2.0 numpy==2.5.5 packaging==26.4 certifi==2026.11.20 pandas==3.1.1
  shapely==2.2.0 xarray==2026.9.0)
# Processes of a pytest run that call one of the functions proj_count.c counts: pytest itself,
# and the `pyproj` commands test_cli starts.
MIN_CALLERS=1
root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/pyproj}
mkdir -p "$work"
work=$(cd "$work" && pwd)
. "$root/tools/clients/libproj_proof.sh"

[ -d "$ref/src/.git" ] || { echo "no reference PROJ source in $ref/src: run tools/build-reference.sh" >&2; exit 1; }

packages=(cmake g++ libtiff-dev libcurl4-openssl-dev libsqlite3-dev python3-venv python3-dev)
if [ "$(id -u)" = 0 ] && command -v apt-get > /dev/null; then
  missing=()
  for p in "${packages[@]}"; do dpkg -s "$p" > /dev/null 2>&1 || missing+=("$p"); done
  [ ${#missing[@]} = 0 ] || { apt-get update -q > /dev/null; apt-get install -y -q --no-install-recommends "${missing[@]}" > /dev/null; }
fi

# PROJ 9.9.0 with TIFF and curl, from the source of the reference (same tag, no new download).
ref_tc=${PROJ_REFERENCE_TIFF_CURL:-$root/target/proj-reference-tiff-curl}
if [ ! -x "$ref_tc/build/bin/proj" ]; then
  mkdir -p "$ref_tc"
  [ -d "$ref_tc/src/.git" ] || git -c advice.detachedHead=false clone -q "$ref/src" "$ref_tc/src"
  [ "$(git -C "$ref_tc/src" describe --tags)" = 9.9.0 ] || { echo "$ref_tc/src is not PROJ 9.9.0" >&2; exit 1; }
  (cd "$root" && PROJ_REF_TIFF_CURL=ON tools/build-reference.sh "$ref_tc" > /dev/null)
fi
for opt in ENABLE_TIFF ENABLE_CURL; do
  grep -q "^$opt:BOOL=ON$" "$ref_tc/build/CMakeCache.txt" || { echo "$ref_tc: PROJ built without $opt" >&2; exit 1; }
done

sdist=$work/pyproj-$PYPROJ_VERSION.tar.gz
if [ ! -s "$sdist" ]; then
  url=$(curl -fsSL "https://pypi.org/pypi/pyproj/$PYPROJ_VERSION/json" \
    | python3 -I -c "import json,sys; print([u['url'] for u in json.load(sys.stdin)['urls'] if u['packagetype']=='sdist'][0])")
  curl -fsSL "$url" -o "$sdist"
fi
echo "$PYPROJ_SHA256  $sdist" | sha256sum --check --quiet
rm -rf "$work/src" && mkdir -p "$work/src" && tar -xzf "$sdist" -C "$work/src"
tests=$work/src/pyproj-$PYPROJ_VERSION

rm -rf "$work/ref-prefix" "$work/ours-prefix"
cmake --install "$ref_tc/build" --prefix "$work/ref-prefix" > /dev/null
PROJ_RS_CAPI_FEATURES=network "$root/capi/tests/dist.sh" --release "$work/ours-prefix" > /dev/null
data=$work/ref-prefix/share/proj
exports=$(nm -D --defined-only "$work/ours-prefix/lib/libproj.so.25")
grep -q ' geod_init$' <<< "$exports" || { echo "libproj of proj-rs exports no geod_init" >&2; exit 1; }
proof_build "$work" "$work/ref-prefix/include"
proof_distinct "$work/ref-prefix/lib" "$work/ours-prefix/lib"

code=$(curl -s -o /dev/null -w '%{http_code}' https://cdn.proj.org/files.geojson || true)
case "$code" in
  200) echo "cdn.proj.org reachable (the tests marked network use it)" ;;
  *) echo "cdn.proj.org NOT reachable (HTTP $code): the tests marked network fail on both sides" ;;
esac

run() { # run <name> <prefix>
  local name=$1 prefix=$2 out=$work/$1 log=$work/projcount-$1.log
  rm -rf "$out" "$log" && mkdir -p "$out"
  python3 -m venv "$out/venv"
  "$out/venv/bin/pip" install -q "${PACKAGES[@]}" > "$out/pip.log" 2>&1
  PROJ_DIR=$prefix PROJ_VERSION=9.9.0 "$out/venv/bin/pip" install -v --no-binary pyproj --no-cache-dir \
    --no-deps "$sdist" >> "$out/pip.log" 2>&1 || { echo "$name: pyproj does not build ($out/pip.log)"; tail -5 "$out/pip.log"; return 1; }
  cp -r "$tests/test" "$tests/pytest.ini" "$out/"
  local site mods=()
  site=$(PROJ_DATA="$data" "$out/venv/bin/python" -c 'import pyproj, os; print(os.path.dirname(pyproj.__file__))')
  for so in "$site"/*.so; do
    readelf -d "$so" | grep -q 'NEEDED.*\[libproj\.so\.25\]' && mods+=("$so")
  done
  [ ${#mods[@]} -ge 5 ] || { echo "$name: only ${#mods[@]} pyproj modules link libproj.so.25" >&2; return 1; }
  (cd "$out" && proof_static "$name" "$prefix/lib" "$prefix/lib" "${mods[@]}")
  (cd "$out" && export PROJ_DATA="$data" PROJ_NETWORK=OFF PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
    HOME="$out" && proof_call "$name" "$prefix/lib" "$prefix/lib" "$out/venv/bin/python" -c \
    'from pyproj import Transformer; print(Transformer.from_crs("EPSG:4326", "EPSG:32631").transform(45, 3))')
  (cd "$out" && env $(proof_env "$prefix/lib" "$log") PATH="$out/venv/bin:$PATH" PROJ_DATA="$data" \
    PROJ_NETWORK=OFF PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES HOME="$out" \
    "$out/venv/bin/python" -m pytest test -q -p no:cacheprovider -rfEs \
    --junitxml="$out/results.xml" -o junit_family=xunit1 > "$out/pytest.log" 2>&1) || true
  if [ ! -s "$out/results.xml" ]; then
    echo "$name: pytest stopped without a report, a crash (see $out/pytest.log):"
    grep -m1 -A3 "Fatal Python error" "$out/pytest.log" || tail -5 "$out/pytest.log"
    return 1
  fi
  proof_log "$name" "$prefix/lib" "$log" "$MIN_CALLERS"
  echo "$name: $(tail -1 "$out/pytest.log")"
  grep -hE '^(FAILED|ERROR|SKIPPED)' "$out/pytest.log" | sed "s/^/$name: /" || true
}
echo "pyproj $PYPROJ_VERSION, $(python3 --version), ${PACKAGES[*]}"
echo "reference: $("$work/ref-prefix/bin/projinfo" 2>&1 | head -1), TIFF and curl"
run reference "$work/ref-prefix"
run proj-rs "$work/ours-prefix"
echo
python3 -I "$root/tools/clients/compare_junit.py" "$work/reference/results.xml" "$work/proj-rs/results.xml"
echo
echo "full list: $work/proj-rs/results.xml.regressions.tsv"
