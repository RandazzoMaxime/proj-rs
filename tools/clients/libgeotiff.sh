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
set -euo pipefail

TAG=1.7.4
COMMIT=96024f677642486f97cac43659bef57f4ed0590b
root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/libgeotiff}
mkdir -p "$work"
work=$(cd "$work" && pwd)

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
export PROJ_DATA=$prefix/share/proj PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES PROJ_NETWORK=OFF

bin=$work/build/bin
run() {
  rm -rf "$work/run-$1" && cp -r "$work/src/libgeotiff/test" "$work/run-$1"
  (cd "$work/run-$1" && sh ./testlistgeo "$bin/listgeo" > listgeo.log 2>&1 || true
   sh ./testgeotifcp "$bin/geotifcp" "$bin/listgeo" > geotifcp.log 2>&1 || true)
  echo "$1: $(grep -c '^Testing listgeo' "$work/run-$1/testlistgeo_out") files listed," \
    "$(grep -c '^Corner Coordinates' "$work/run-$1/testlistgeo_out") with corners"
}
run ref
LD_LIBRARY_PATH=$work/ours/lib run ours

# The expected outputs of the test suite were written with an older EPSG database: the outputs of
# the two libraries are compared to each other.
diff "$work/run-ref/testlistgeo_out" "$work/run-ours/testlistgeo_out" > "$work/listgeo.diff" \
  || { echo "listgeo differs: $work/listgeo.diff" >&2; exit 1; }
diff "$work/run-ref/geotifcp.log" "$work/run-ours/geotifcp.log" > "$work/geotifcp.diff" \
  || { echo "geotifcp differs: $work/geotifcp.diff" >&2; exit 1; }
echo "listgeo and geotifcp write the same output with both libraries"
