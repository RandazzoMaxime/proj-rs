#!/usr/bin/env bash
# Run the test suite of pyproj, a real client of libproj, against PROJ 9.9.0 and against the C
# library of proj-rs, and list what fails with proj-rs only (docs/CLIENTS-pyproj.md).
#
# Usage: tools/clients/pyproj.sh [WORKDIR]          (default: target/clients/pyproj)
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), python3
# with venv and its headers, a C compiler, network access to PyPI. Both runs are offline
# (-m "not network") and read the same data directory, the share/proj of the reference install,
# so a difference comes from the library. The pyproj source is pinned by its sha256.
#
# GEODESIC_SHIM: the C library of proj-rs does not export geodesic.h (geod_*) yet, and pyproj does
# not compile without it. Until it does, the script builds PROJ's own geodesic.c into a separate
# library, preloaded in the proj-rs run only, so the rest of pyproj can be measured. It says so.
set -euo pipefail

PYPROJ_VERSION=3.8.0
PYPROJ_SHA256=efa59725bba68bf97fa808b61302df32934acdceb6a5c92a8dd0e71dc266a876
root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/pyproj}
mkdir -p "$work"
work=$(cd "$work" && pwd)

[ -x "$ref/build/bin/proj" ] || { echo "no reference PROJ in $ref: run tools/build-reference.sh" >&2; exit 1; }

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
cmake --install "$ref/build" --prefix "$work/ref-prefix" > /dev/null
"$root/capi/tests/dist.sh" --release "$work/ours-prefix" > /dev/null
data=$work/ref-prefix/share/proj

preload=
if ! nm -D --defined-only "$work/ours-prefix/lib/libproj.so.25" | grep -q ' geod_init$'; then
  echo "NOTE: libproj of proj-rs exports no geod_*: PROJ's geodesic.c is preloaded in the proj-rs run"
  cp "$ref/src/src/geodesic.h" "$work/ours-prefix/include/"
  cc -O2 -fPIC -shared "$ref/src/src/geodesic.c" -o "$work/ours-prefix/libgeodesic-shim.so" -lm
  preload=$work/ours-prefix/libgeodesic-shim.so
fi

run() { # run <name> <prefix> <preload>
  local name=$1 prefix=$2 shim=$3 out=$work/$1
  rm -rf "$out" && mkdir -p "$out"
  python3 -m venv "$out/venv"
  "$out/venv/bin/pip" install -q pytest numpy > "$out/pip.log" 2>&1
  PROJ_DIR=$prefix PROJ_VERSION=9.9.0 "$out/venv/bin/pip" install -v --no-binary pyproj --no-cache-dir \
    "$sdist" >> "$out/pip.log" 2>&1 || { echo "$name: pyproj does not build ($out/pip.log)"; tail -5 "$out/pip.log"; return 0; }
  cp -r "$tests/test" "$tests/pytest.ini" "$out/"
  (cd "$out" && env PROJ_DATA="$data" LD_LIBRARY_PATH="$prefix/lib" LD_PRELOAD="$shim" PROJ_NETWORK=OFF \
    PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES HOME="$out" "$out/venv/bin/python" -m pytest test \
    -m "not network" -q -p no:cacheprovider --continue-on-collection-errors \
    --junitxml="$out/results.xml" -o junit_family=xunit1 > "$out/pytest.log" 2>&1) || true
  if [ ! -s "$out/results.xml" ]; then
    echo "$name: pytest stopped without a report, a crash (see $out/pytest.log):"
    grep -m1 -A3 "Fatal Python error" "$out/pytest.log" || tail -5 "$out/pytest.log"
    return 1
  fi
  echo "$name: $(tail -1 "$out/pytest.log")"
}
run reference "$work/ref-prefix" "" || exit 1
run proj-rs "$work/ours-prefix" "$preload" || exit 1
echo
python3 -I "$root/tools/clients/compare_junit.py" "$work/reference/results.xml" "$work/proj-rs/results.xml"
echo
echo "full list: $work/proj-rs/results.xml.regressions.tsv"
