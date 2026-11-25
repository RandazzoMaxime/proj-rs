#!/usr/bin/env bash
# Run the test suite of GMT (Generic Mapping Tools), a real client of libproj, against PROJ 9.9.0
# and against proj-rs (docs/CLIENTS-gmt.md).
#
# GMT reaches PROJ only through GDAL: map_init_proj4 lets GDAL's OGRCoordinateTransformation do
# every -J+proj=... projection, gmt_ogrproj.c converts coordinates when read or written with a
# different SRS, and gmt_gdalread.c computes lat/lon extents of rasters with one. Its own plotting
# projections (gmt_proj.c) never call libproj, so what this suite adds is the GDAL side of the
# library: PROJ strings created through OCTNewCoordinateTransformation, WKT parsed when a GeoTIFF
# is opened, SRS written back out.
#
# Every test script renders PostScript; the harness of GMT, gmtest, compares it against a baseline.
# Those baselines live in a DVC remote that answers 404 today, so most tests end Failed on both
# sides. This script does not care: it compares the two passes with each other instead, test by
# test (outcome of the ctest run), and file by file inside the directory each test works in
# (byte for byte, the PostScript rendered with GraphicsMagick). A file that differs is accepted
# only if a rerun of the same test with the reference library also produces two different files:
# GMT is not deterministic in a few spots under a parallel load (docs/CLIENTS-gmt.md).
#
# Usage: tools/clients/gmt.sh [WORKDIR]      (default: target/clients/gmt)
# GMT_DEPS_PREFIX adds a prefix to CMAKE_PREFIX_PATH for NetCDF, GDAL and friends when they are
# not in the system paths (for instance a home directory).
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), cmake, a C
# and C++ compiler, and as root: libnetcdf-dev, libgdal-dev, zlib, plus for the tests: perl,
# graphicsmagick, ghostscript. GSHHG and DCW are downloaded from their GitHub releases (gh).
# The source is pinned by its commit. GMT is built once against PROJ 9.9.0: its binaries never
# link libproj, the GDAL it links does, and the pass swaps the library with LD_LIBRARY_PATH
# (same soname).

set -euo pipefail

TAG=6.5.0
COMMIT=6da8ee2db90cb8e5dc0f2b6e80c875c4fd38e433
GSHHG_VERSION=2.3.7
DCW_VERSION=2.2.0

root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/gmt}
mkdir -p "$work"
work=$(cd "$work" && pwd)

[ -x "$ref/build/bin/cs2cs" ] || { echo "no reference PROJ in $ref: run tools/build-reference.sh" >&2; exit 1; }
command -v gdal-config >/dev/null || { echo "need libgdal-dev (gdal-config) on the PATH" >&2; exit 1; }
command -v gm >/dev/null || { echo "need graphicsmagick (gm) on the PATH" >&2; exit 1; }
command -v gs >/dev/null || { echo "need ghostscript (gs) on the PATH" >&2; exit 1; }

rm -rf "$work/src"
git clone -q --depth 1 --branch "$TAG" https://github.com/GenericMappingTools/gmt.git "$work/src" 2>/dev/null
[ "$(git -C "$work/src" rev-parse HEAD)" = "$COMMIT" ] || { echo "$TAG is not $COMMIT" >&2; exit 1; }

gh release download --repo GenericMappingTools/gshhg-gmt --pattern "gshhg-gmt-$GSHHG_VERSION.tar.gz" --dir "$work" --clobber
gh release download --repo GenericMappingTools/dcw-gmt --pattern "dcw-gmt-$DCW_VERSION.tar.gz" --dir "$work" --clobber
mkdir -p "$work/data"
tar -xzf "$work/gshhg-gmt-$GSHHG_VERSION.tar.gz" -C "$work/data"
tar -xzf "$work/dcw-gmt-$DCW_VERSION.tar.gz" -C "$work/data"

# The test data of GMT lives in a DVC remote. Pull it when it works (it makes gmtest compare the
# PostScript against real baselines and the failure count go down); the comparison between the
# two passes does not depend on it, so a missing baseline fails the test on both sides.
if command -v dvc >/dev/null; then
  (cd "$work/src" && dvc pull -j 4 $(find test/baseline doc/scripts/images -name '*.dvc' 2>/dev/null) 2>/dev/null) || true
fi

prefix=$work/ref-prefix
rm -rf "$prefix"
cmake --install "$ref/build" --prefix "$prefix" > /dev/null

# GMT never calls enable_testing(); its tests are listed by add_test but ctest needs the command.
mkdir -p "$work/cmake-tmp"
echo 'enable_testing()' > "$work/cmake-tmp/enable.cmake"

build=$work/build
rm -rf "$build"
cmake -S "$work/src" -B "$build" -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_PREFIX_PATH="$prefix${GMT_DEPS_PREFIX:+;$GMT_DEPS_PREFIX}" \
  -DGSHHG_ROOT="$work/data/gshhg-gmt-$GSHHG_VERSION" -DDCW_ROOT="$work/data/dcw-gmt-$DCW_VERSION" \
  -DDO_TESTS=ON -DDO_API_TESTS=ON -DDO_SUPPLEMENT_TESTS=ON -DDO_EXAMPLES=OFF \
  -DCMAKE_PROJECT_INCLUDE="$work/cmake-tmp/enable.cmake" > "$work/cmake.log" 2>&1
cmake --build "$build" --parallel "$(nproc 2>/dev/null || echo 4)" > "$work/make.log" 2>&1

"$root/capi/tests/dist.sh" --release "$work/ours" > /dev/null

# The suite of GMT downloads remote datasets on demand in some tests; without the network they
# fail, the same way in both passes. LDFLAGS pins gmt's rpath to the build dir.
export PROJ_DATA="$prefix/share/proj:$root/data/tests" PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES
export PATH="$prefix/bin:$PATH"

run() {
  local name=$1 libdir=$2
  local env=(LD_LIBRARY_PATH="$libdir:${LD_LIBRARY_PATH:-}" PATH="$PATH" PROJ_DATA="$PROJ_DATA"
    PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES)
  rm -rf "$build/Testing" "$build/test/fail_count.txt"
  (cd "$build" && env "${env[@]}" ctest -j "$(nproc 2>/dev/null || echo 4)" \
    -O "$work/ctest-$name.log" > "$work/ctest-$name-run.log" 2>&1 || true)
  # Keep every file each test wrote: gmtest removes the working directory of a passing test, but a
  # Failed test (no baseline, here) leaves it in place.
  rm -rf "$work/outputs-$name"
  mkdir -p "$work/outputs-$name"
  cp -a "$build/test" "$work/outputs-$name/test"
  cp -a "$build/doc" "$work/outputs-$name/doc"
  find "$work/outputs-$name" \( -name 'CMakeFiles' \) -prune -exec rm -rf {} + 2>/dev/null || true
}

run ref "$prefix/lib"
run ours "$work/ours/lib"

# Compare the two passes: the outcome of each ctest entry first, then every written file.
python3 "$(dirname "$0")/gmt_compare.py" "$work" "$build" "$(command -v gm)" "$prefix/lib" "$work/ours/lib"
