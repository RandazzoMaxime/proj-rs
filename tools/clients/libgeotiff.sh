#!/usr/bin/env bash
# Run the tests of libgeotiff, a real client of libproj, against PROJ 9.9.0 and against the C
# library of proj-rs, and compare what listgeo and geotifcp write (docs/CLIENTS-libgeotiff.md).
#
# Usage: tools/clients/libgeotiff.sh [WORKDIR]      (default: target/clients/libgeotiff)
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), cmake, a C
# compiler, libtiff (Debian: libtiff-dev), network access to github.com. The source is pinned by
# its commit. libgeotiff is built once against PROJ 9.9.0; the second run swaps in the libproj of
# proj-rs through LD_LIBRARY_PATH (same soname, RUNPATH). Both runs read the same data directory.
#
# testlistgeo prints the corners of 46 of its 48 files: it runs listgeo with -no_corners on the
# two Cassini-Soldner files (testlistgeo, lines 62 and 66). So each pass also runs
# `listgeo -proj4` on every file of test/data, corners included, and those outputs are compared
# too. Which libproj each pass loads is proven by libproj_proof.sh (ldd, no DT_RPATH,
# LD_DEBUG=libs, the proj_count.c interposer for the whole pass).
set -euo pipefail

TAG=1.7.4
COMMIT=96024f677642486f97cac43659bef57f4ed0590b
root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/libgeotiff}
mkdir -p "$work"
work=$(cd "$work" && pwd)
. "$root/tools/clients/libproj_proof.sh"
# Processes of a pass that call into libproj (listgeo runs that print corners); a pass makes 100.
MIN_CALLERS=90

[ -x "$ref/build/bin/proj" ] || { echo "no reference PROJ in $ref: run tools/build-reference.sh" >&2; exit 1; }

rm -rf "$work/src"
git clone -q --depth 1 --branch "$TAG" https://github.com/OSGeo/libgeotiff.git "$work/src" 2>/dev/null
[ "$(git -C "$work/src" rev-parse HEAD)" = "$COMMIT" ] || { echo "$TAG is not $COMMIT" >&2; exit 1; }

prefix=$work/ref-prefix
rm -rf "$prefix" "$work/build"
cmake --install "$ref/build" --prefix "$prefix" > /dev/null
# geotifcp includes proj.h without the include directory of PROJ: CMAKE_C_FLAGS gives it.
cmake -S "$work/src/libgeotiff" -B "$work/build" -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH="$prefix" \
  -DCMAKE_C_FLAGS="-I$prefix/include" -DCMAKE_BUILD_RPATH="$prefix/lib" -DWITH_JPEG=OFF \
  -DCMAKE_EXE_LINKER_FLAGS=-Wl,--enable-new-dtags -DCMAKE_SHARED_LINKER_FLAGS=-Wl,--enable-new-dtags \
  > "$work/cmake.log" 2>&1
cmake --build "$work/build" -j"$(nproc)" > "$work/build.log" 2>&1
"$root/capi/tests/dist.sh" "$work/ours" > /dev/null
proof_build "$work" "$prefix/include"
proof_distinct "$prefix/lib" "$work/ours/lib"
export PROJ_DATA=$prefix/share/proj PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES PROJ_NETWORK=OFF

bin=$work/build/bin
run() {
  local name=$1 libdir=$2 log=$work/projcount-$1.log dir=$work/run-$1
  rm -rf "$dir" "$log" && cp -r "$work/src/libgeotiff/test" "$dir"
  (cd "$work" && proof_static "$name" "$libdir" "$libdir" "$bin/listgeo" "$bin/geotifcp" \
    "$(ls "$work"/build/lib/libgeotiff.so.* | head -1)")
  (cd "$work" && proof_call "$name" "$libdir" "$libdir" "$bin/listgeo" "$dir/data/byte.tif")
  (cd "$dir" && export $(proof_env "$libdir" "$log") &&
   for f in data/*.tif; do echo "== $f"; "$bin/listgeo" -proj4 "$f" 2>&1 || true; done > all-corners.out &&
   { sh ./testlistgeo "$bin/listgeo" > listgeo.log 2>&1 || true; } &&
   { sh ./testgeotifcp "$bin/geotifcp" "$bin/listgeo" > geotifcp.log 2>&1 || true; })
  proof_log "$name" "$libdir" "$log" "$MIN_CALLERS"
  echo "$name: $(grep -c '^Testing listgeo' "$dir/testlistgeo_out") files listed by testlistgeo," \
    "$(grep -c '^Corner Coordinates' "$dir/testlistgeo_out") with corners;" \
    "listgeo -proj4: $(grep -c '^== ' "$dir/all-corners.out") files," \
    "$(grep -c '^Corner Coordinates' "$dir/all-corners.out") with corners"
}
run ref "$prefix/lib"
run ours "$work/ours/lib"

# The expected outputs of the test suite were written with an older EPSG database: the outputs of
# the two libraries are compared to each other.
diff "$work/run-ref/testlistgeo_out" "$work/run-ours/testlistgeo_out" > "$work/listgeo.diff" \
  || { echo "listgeo differs: $work/listgeo.diff" >&2; exit 1; }
diff "$work/run-ref/geotifcp.log" "$work/run-ours/geotifcp.log" > "$work/geotifcp.diff" \
  || { echo "geotifcp differs: $work/geotifcp.diff" >&2; exit 1; }
diff "$work/run-ref/all-corners.out" "$work/run-ours/all-corners.out" > "$work/corners.diff" \
  || { echo "listgeo -proj4 differs: $work/corners.diff" >&2; exit 1; }
echo "listgeo and geotifcp write the same output with both libraries"
