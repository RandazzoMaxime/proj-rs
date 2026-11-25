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
# (gmt_compare.py). gmtest removes the directory of a test that passes; the copy of gmtest in the
# build tree is patched to keep it, so the files of passing tests are compared too (the verdict
# of gmtest is unchanged).
#
# Usage: tools/clients/gmt.sh [WORKDIR]      (default: target/clients/gmt)
# GMT_DEPS_PREFIX adds a prefix to CMAKE_PREFIX_PATH for NetCDF, GDAL and friends when they are
# not in the system paths (for instance a home directory).
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), cmake, a C
# and C++ compiler, curl, and as root: libnetcdf-dev, libgdal-dev, zlib, flex, plus for the tests:
# perl, graphicsmagick, ghostscript, netcdf-bin (ncdump). GSHHG and DCW are downloaded from their
# GitHub releases and checked against their sha256. The GMT source is pinned by its commit. With
# the network, the tests that use remote datasets (@earth_relief_...) download them from the GMT
# data server in each pass (an empty GMT user directory per pass); without it, they fail in both
# passes. The GDAL of the system is used (its version goes to WORKDIR/versions.txt).
#
# Which libproj runs: GMT's binaries do not link libproj; they link the GDAL of the system, which
# links libproj.so.25 (the soname of PROJ 9.9.0 too, and of the C library of proj-rs). GMT is
# built once, and each pass puts its libproj first on LD_LIBRARY_PATH. Nothing else proves that
# swap, so check_libs below fails the script unless, for each pass: ldd resolves libproj.so.25 of
# the gmt binary to that pass's file, no object of the chain carries a DT_RPATH that would be
# searched before LD_LIBRARY_PATH (GMT is linked with --enable-new-dtags, as pdal.sh does, so its
# build path is a DT_RUNPATH), and a real gmt call that reaches PROJ maps that file (LD_DEBUG=libs
# and the proj_count.c interposer). The interposer then stays preloaded for the whole ctest pass:
# every process that maps a libproj logs its path, and the pass fails if one is not the expected
# file. It also counts the calls into libproj (docs/CLIENTS-gmt.md).

set -euo pipefail

TAG=6.5.0
COMMIT=6da8ee2db90cb8e5dc0f2b6e80c875c4fd38e433
GSHHG_VERSION=2.3.7
GSHHG_SHA256=9bb1a956fca0718c083bef842e625797535a00ce81f175df08b042c2a92cfe7f
DCW_VERSION=2.2.0
DCW_SHA256=f2a8a7b7365bdd17269aa1d412966a871528eefa9b2a7409815832a702ff7dcb
# A gmt process maps libproj (through libgdal) even when it never calls it; a pass of ~1180
# scripts runs many thousands of them. Fewer than this many means the interposer did not log.
MIN_GMT_PROCESSES=2000

root=$(cd "$(dirname "$0")/../.." && pwd)
here=$root/tools/clients
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/gmt}
mkdir -p "$work"
work=$(cd "$work" && pwd)
start=$(date +%s)

[ -x "$ref/build/bin/cs2cs" ] || { echo "no reference PROJ in $ref: run tools/build-reference.sh" >&2; exit 1; }
for tool in gdal-config gm gs ncdump curl readelf ldd sha256sum cc; do
  command -v "$tool" >/dev/null || { echo "need $tool on the PATH" >&2; exit 1; }
done

rm -rf "$work/src"
git clone -q --depth 1 --branch "$TAG" https://github.com/GenericMappingTools/gmt.git "$work/src" 2>/dev/null
[ "$(git -C "$work/src" rev-parse HEAD)" = "$COMMIT" ] || { echo "$TAG is not $COMMIT" >&2; exit 1; }

fetch() {
  local repo=$1 version=$2 sum=$3 file=$4
  [ -f "$work/$file" ] && echo "$sum  $work/$file" | sha256sum -c --quiet 2>/dev/null && return
  curl -fsSL -o "$work/$file" "https://github.com/GenericMappingTools/$repo/releases/download/$version/$file"
  echo "$sum  $work/$file" | sha256sum -c --quiet
}
fetch gshhg-gmt "$GSHHG_VERSION" "$GSHHG_SHA256" "gshhg-gmt-$GSHHG_VERSION.tar.gz"
fetch dcw-gmt "$DCW_VERSION" "$DCW_SHA256" "dcw-gmt-$DCW_VERSION.tar.gz"
rm -rf "$work/data"
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
  -DCMAKE_EXE_LINKER_FLAGS=-Wl,--enable-new-dtags -DCMAKE_SHARED_LINKER_FLAGS=-Wl,--enable-new-dtags \
  -DCMAKE_MODULE_LINKER_FLAGS=-Wl,--enable-new-dtags \
  -DCMAKE_PROJECT_INCLUDE="$work/cmake-tmp/enable.cmake" > "$work/cmake.log" 2>&1
cmake --build "$build" --parallel "$(nproc 2>/dev/null || echo 4)" > "$work/make.log" 2>&1

# Keep the working directory of every test, passed or not (the verdict is left alone).
keep='test "$ERROR" -eq 0 -a "$memtrack_err" -eq 0 -a "$valgrind_err" -eq 0 && rm -rf "$exec_dir"'
[ "$(grep -cF "$keep" "$build/test/gmtest")" = 1 ] || { echo "gmtest: cleanup line not found" >&2; exit 1; }
sed -i "s|&& rm -rf \"\$exec_dir\"|\&\& : kept for tools/clients/gmt.sh \"\$exec_dir\"|" "$build/test/gmtest"
[ "$(grep -cF "$keep" "$build/test/gmtest")" = 0 ] || { echo "gmtest: patch did not apply" >&2; exit 1; }

"$root/capi/tests/dist.sh" --release "$work/ours" > /dev/null
cc -shared -fPIC -O2 -Wall -Wextra -I"$prefix/include" "$here/proj_count.c" -o "$work/proj_count.so" -ldl

{
  echo "GMT $TAG ($COMMIT), GSHHG $GSHHG_VERSION, DCW $DCW_VERSION"
  echo "GDAL $(gdal-config --version): $(readlink -f "$(ldd "$build/src/gmt" | awk '$1 ~ /^libgdal/ { print $3 }')")"
  echo "libproj of the system (not used by either pass): $(ldconfig -p | awk '$1 == "libproj.so.25" { print $NF; exit }' | xargs -r readlink -f)"
  echo "Ghostscript $(gs --version), $(gm version | head -1)"
  echo "$(uname -srm), $(nproc) CPU"
} > "$work/versions.txt"
cat "$work/versions.txt"

export PROJ_DATA="$prefix/share/proj:$root/data/tests" PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES
export PATH="$prefix/bin:$PATH"

# The environment of a pass: its libproj first, the interposer, and its own GMT user directory.
pass_env() {
  local name=$1 libdir=$2 log=$3
  echo "LD_LIBRARY_PATH=$libdir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  echo "GMT_USERDIR=$work/userdir"
  echo "LD_PRELOAD=$work/proj_count.so"
  echo "PROJ_COUNT_LOG=$log"
}

# Fail unless a gmt process of this pass loads $libdir/libproj.so.25 (see the header).
check_libs() {
  local name=$1 libdir=$2 want obj path rpath dir got
  want=$(readlink -f "$libdir/libproj.so.25")
  [ -f "$want" ] || { echo "$name: no $libdir/libproj.so.25" >&2; exit 1; }
  local ldd_out
  ldd_out=$(env LD_LIBRARY_PATH="$libdir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}" ldd "$build/src/gmt")
  got=$(awk '$1 == "libproj.so.25" { print $3 }' <<< "$ldd_out")
  [ -n "$got" ] && [ "$(readlink -f "$got")" = "$want" ] \
    || { echo "$name: ldd resolves libproj.so.25 to '$got', not $want" >&2; exit 1; }
  echo "$name: ldd gmt: libproj.so.25 => $got"
  for obj in "$build/src/gmt" $(awk '$2 == "=>" && $3 ~ /^\// { print $3 }' <<< "$ldd_out"); do
    rpath=$(readelf -d "$obj" | sed -n 's/.*(RPATH).*\[\(.*\)\]/\1/p')
    [ -z "$rpath" ] && continue
    case "$(basename "$obj")" in
      gmt|libgmt.so*|libpostscriptlight.so*|libgdal.so*)
        echo "$name: $obj has a DT_RPATH ($rpath), searched before LD_LIBRARY_PATH" >&2; exit 1 ;;
    esac
    for dir in ${rpath//:/ }; do
      dir=${dir//\$ORIGIN/$(dirname "$obj")}
      if [ -e "$dir/libproj.so.25" ]; then
        echo "$name: $obj has a DT_RPATH to $dir, which holds a libproj.so.25" >&2; exit 1
      fi
    done
  done
  echo "$name: no DT_RPATH that could load another libproj (gmt: $(readelf -d "$build/src/gmt" | grep -oE '\((RUNPATH|RPATH)\)' | tr -d '()' | head -1 || true))"
  local log=$work/check-$name.log out
  rm -f "$log"
  out=$(cd "$work" && echo "3 45" | env $(pass_env "$name" "$libdir" "$log") LD_DEBUG=libs \
    "$build/src/gmt" mapproject "-J+proj=utm +zone=31 +datum=WGS84 +units=m" -R0/6/40/50 2>"$work/check-$name.ld")
  echo "$name: gmt mapproject -J+proj=utm: 3 45 -> $out"
  got=$(sed -n 's/.*calling init: \(.*libproj\.so[^ ]*\)$/\1/p' "$work/check-$name.ld" | head -1)
  [ -n "$got" ] && [ "$(readlink -f "$got")" = "$want" ] \
    || { echo "$name: LD_DEBUG=libs shows libproj init from '$got', not $want" >&2; exit 1; }
  echo "$name: LD_DEBUG=libs: calling init: $got"
  awk -F'\t' -v want="$want" -v name="$name" '
    $1 == "map" { maps++; if ($5 != want) { print name ": process maps " $5 > "/dev/stderr"; bad++ } }
    $1 == "calls" && $6 ~ /proj_trans/ { trans++; line = $6 }
    END {
      if (bad || maps == 0 || trans == 0) { print name ": the probe did not reach " want > "/dev/stderr"; exit 1 }
      print name ": interposer: " want " mapped, " line
    }' "$log"
}

run() {
  local name=$1 libdir=$2
  local log=$work/projcount-$name.log want
  want=$(readlink -f "$libdir/libproj.so.25")
  # Each pass starts from an empty GMT user directory, at the same path (it is written in the
  # history of the grids): the remote datasets (@earth_relief_...) are downloaded again and
  # converted through GDAL, which calls libproj; a cache filled by the first pass would spare
  # the second one those calls. GMT ignores a GMT_USERDIR that does not exist (~/.gmt then).
  rm -rf "$work/userdir"
  mkdir -p "$work/userdir"
  [ "$(env GMT_USERDIR="$work/userdir" "$build/src/gmt" --show-userdir)" = "$work/userdir" ] \
    || { echo "$name: gmt does not use GMT_USERDIR" >&2; exit 1; }
  check_libs "$name" "$libdir"
  rm -rf "$build/Testing" "$build/test/fail_count.txt" "$log"
  local t0
  t0=$(date +%s)
  (cd "$build" && env $(pass_env "$name" "$libdir" "$log") PATH="$PATH" PROJ_DATA="$PROJ_DATA" \
    PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES ctest -j "$(nproc 2>/dev/null || echo 4)" \
    -O "$work/ctest-$name.log" > "$work/ctest-$name-run.log" 2>&1 || true)
  echo "$name: ctest $(( $(date +%s) - t0 )) s, $(grep -E 'tests passed' "$work/ctest-$name.log" || echo 'no summary')"
  # Every process of the pass that mapped a libproj must have mapped this one.
  awk -F'\t' -v want="$want" -v gmt="$build/src/gmt" -v name="$name" -v min="$MIN_GMT_PROCESSES" '
    $1 == "map" { maps++; if ($3 == gmt) gmts++; if ($5 != want) { other[$5]++; bad++ } }
    END {
      for (p in other) print name ": " other[p] " processes mapped " p > "/dev/stderr"
      if (bad) exit 1
      if (gmts < min) { print name ": only " gmts+0 " gmt processes logged (floor " min ")" > "/dev/stderr"; exit 1 }
      print name ": " maps " processes mapped a libproj (" gmts " of them gmt), all " want
    }' "$log"
  # Keep every file each test wrote.
  rm -rf "$work/outputs-$name"
  mkdir -p "$work/outputs-$name"
  cp -a "$build/test" "$work/outputs-$name/test"
  cp -a "$build/doc" "$work/outputs-$name/doc"
  find "$work/outputs-$name" \( -name 'CMakeFiles' \) -prune -exec rm -rf {} + 2>/dev/null || true
}

ref_lib=$(readlink -f "$prefix/lib/libproj.so.25")
ours_lib=$(readlink -f "$work/ours/lib/libproj.so.25")
[ "$(sha256sum < "$ref_lib")" != "$(sha256sum < "$ours_lib")" ] \
  || { echo "the two libproj.so.25 are the same file" >&2; exit 1; }
expected_tests=$(cd "$build" && ctest -N | sed -n 's/^Total Tests: //p')
echo "ctest lists $expected_tests tests"

run ref "$prefix/lib"
run ours "$work/ours/lib"

# Compare the two passes: the outcome of each ctest entry first, then every written file, then
# the calls each test made into libproj.
python3 "$here/gmt_compare.py" "$work" "$build" "$(command -v gm)" "$prefix/lib" "$work/ours/lib" \
  "$expected_tests" "$work/proj_count.so"
echo "total: $(( $(date +%s) - start )) s"
