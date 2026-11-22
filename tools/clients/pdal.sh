#!/usr/bin/env bash
# Run the test suite of PDAL (point clouds), a real client of libproj both directly and through
# GDAL, against PROJ 9.9.0 and against the C library of proj-rs (docs/CLIENTS-pdal.md).
#
# Usage: tools/clients/pdal.sh [WORKDIR]      (default: target/clients/pdal)
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), cmake, a
# C++ compiler, GDAL 3.8 or later with its CMake package (Debian: libgdal-dev), network access to
# github.com. The source is pinned by its commit. PDAL is built once against PROJ 9.9.0; the
# second run swaps in the libproj of proj-rs through LD_LIBRARY_PATH, for PDAL and for GDAL alike
# (same soname). The tests run one at a time: some of them share temporary files.
set -euo pipefail

TAG=2.10.2
COMMIT=27008f6241be44585c866a29dfc13bb16d678dab
root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/pdal}
mkdir -p "$work"
work=$(cd "$work" && pwd)

[ -x "$ref/build/bin/proj" ] || { echo "no reference PROJ in $ref: run tools/build-reference.sh" >&2; exit 1; }

rm -rf "$work/src"
git clone -q --depth 1 --branch "$TAG" https://github.com/PDAL/PDAL.git "$work/src" 2>/dev/null
[ "$(git -C "$work/src" rev-parse HEAD)" = "$COMMIT" ] || { echo "$TAG is not $COMMIT" >&2; exit 1; }

prefix=$work/ref-prefix
rm -rf "$prefix" "$work/build"
cmake --install "$ref/build" --prefix "$prefix" > /dev/null
cmake -S "$work/src" -B "$work/build" -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH="$prefix" \
  -DPROJ_DIR="$prefix/lib/cmake/proj" -DWITH_TESTS=ON -DBUILD_PLUGIN_PGPOINTCLOUD=OFF \
  -DCMAKE_BUILD_RPATH="$prefix/lib" -DCMAKE_EXE_LINKER_FLAGS=-Wl,--enable-new-dtags \
  -DCMAKE_SHARED_LINKER_FLAGS=-Wl,--enable-new-dtags > "$work/cmake.log" 2>&1
cmake --build "$work/build" -j"$(nproc)" > "$work/build.log" 2>&1
"$root/capi/tests/dist.sh" "$work/ours" > /dev/null
export PROJ_DATA=$prefix/share/proj PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES PROJ_NETWORK=OFF

run() {
  (cd "$work/build" && LD_LIBRARY_PATH=$2 ctest -j1 --timeout 300 > "$work/ctest-$1.log" 2>&1 || true)
  grep -E '^[[:space:]]+[0-9]+ - ' "$work/ctest-$1.log" | sed -E 's/^[[:space:]]+//' > "$work/failed-$1.txt" || true
  echo "$1: $(grep -E 'tests passed' "$work/ctest-$1.log")"
}
run ref "$prefix/lib"
run ours "$work/ours/lib"

if diff "$work/failed-ref.txt" "$work/failed-ours.txt"; then
  echo "the same tests fail with both libraries: $(wc -l < "$work/failed-ref.txt")"
else
  echo "the failing tests differ" >&2
  exit 1
fi
