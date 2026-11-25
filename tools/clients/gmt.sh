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
# Every test script renders PostScript; the harness of GMT, gmtest, compares it against a baseline
# image (RMS under 0.003 at 200 dpi). The baselines are those of the 6.5.0 release, checked
# against the md5 of every .dvc file of the tag (gmt_baselines.py). The script then compares the
# two passes with each other, test by test (outcome of the ctest run), and file by file inside the
# directory each test works in (gmt_compare.py). gmtest removes the directory of a test that passes; the copy of gmtest in the
# build tree is patched to keep it, so the files of passing tests are compared too (the verdict
# of gmtest is unchanged).
#
# Usage: tools/clients/gmt.sh [WORKDIR]      (default: target/clients/gmt)
# GMT_DEPS_PREFIX adds a prefix to CMAKE_PREFIX_PATH for NetCDF, GDAL and friends when they are
# not in the system paths (for instance a home directory).
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), cmake, a C
# and C++ compiler, curl, and the packages GMT's CI installs (ci/install-dependencies-linux.sh:
# NetCDF, GDAL, FFTW, PCRE, LAPACK, GLib, GraphicsMagick, Ghostscript, LaTeX, plus gawk), installed
# by the script when it runs as root on Debian or Ubuntu. GSHHG and DCW are downloaded from their
# GitHub releases and checked against their sha256, and so are the baseline images (the
# baseline-images.zip of the release). The GMT source is pinned by its commit. The GDAL of the
# system is used (its version goes to WORKDIR/versions.txt).
#
# The tests run as GMT's CI runs them (.github/workflows/tests.yml): GMT built with the
# configuration of ci/config-gmt-unix.sh, installed, its bin directory on PATH (the scripts of the
# supplements, fzmodeler...), the remote datasets of ci-caches.yml downloaded once from the
# `static` data server before the passes (GMT_DATA_SERVER=static keeps them under
# $HOME/.gmt/static), and `ctest --rerun-failed` twice after the first run. Each pass, and each
# solo rerun of gmt_compare.py, starts from a copy of that downloaded home directory, at the same
# path: a dataset a test needs beyond the list is downloaded within the pass, and four parallel
# tests that fetch the same file at once can read it half written (a truncated JPEG 2000 tile).
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
BASELINES_SHA256=7fff3affa0ed8ec11fd48f2bc27c739fafe2196d96c345b8dec83e1bd2135e6f
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
# The packages of GMT's ci/install-dependencies-linux.sh (RUN_TESTS=true), optional ones included.
packages=(cmake libcurl4-openssl-dev libnetcdf-dev netcdf-bin libgdal-dev gdal-bin zlib1g-dev flex
  perl libfftw3-dev libpcre3-dev liblapack-dev libglib2.0-dev graphicsmagick ghostscript
  texlive-latex-base texlive-binaries texlive-fonts-recommended gawk)
if [ "$(id -u)" = 0 ] && command -v apt-get > /dev/null; then
  missing=()
  for p in "${packages[@]}"; do dpkg -s "$p" > /dev/null 2>&1 || missing+=("$p"); done
  [ ${#missing[@]} = 0 ] || { apt-get update -q > /dev/null; apt-get install -y -q --no-install-recommends "${missing[@]}" > /dev/null; }
fi
for tool in gdal-config gm gs ncdump curl readelf ldd sha256sum cc latex dvips gawk; do
  command -v "$tool" >/dev/null || { echo "need $tool on the PATH" >&2; exit 1; }
done
# gspline_7.sh prints a string with %.1lf: mawk refuses it, gawk (the awk of GMT's CI) does not.
[ "$(readlink -f "$(command -v awk)")" = "$(readlink -f "$(command -v gawk)")" ] \
  || { echo "awk is not gawk: $(readlink -f "$(command -v awk)")" >&2; exit 1; }

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

# The baselines gmtest compares each PostScript with: the DVC remote of GMT (DAGsHub) refuses
# anonymous reads, so they come from the baseline-images.zip of the 6.5.0 release (a `dvc pull`
# of the tag, by GMT's release workflow), pinned by its sha256, and every output is checked
# against the md5 its .dvc file names (gmt_baselines.py).
[ -f "$work/baseline-images.zip" ] && echo "$BASELINES_SHA256  $work/baseline-images.zip" | sha256sum -c --quiet 2>/dev/null \
  || curl -fsSL -o "$work/baseline-images.zip" \
    "https://github.com/GenericMappingTools/gmt/releases/download/$TAG/baseline-images.zip"
echo "$BASELINES_SHA256  $work/baseline-images.zip" | sha256sum -c --quiet
rm -rf "$work/baselines"
mkdir -p "$work/baselines"
(cd "$work/baselines" && python3 -I -m zipfile -e "$work/baseline-images.zip" .)
python3 -I "$here/gmt_baselines.py" "$work/src" "$work/baselines/baseline-images"
rm -rf "$work/baselines"

prefix=$work/ref-prefix
rm -rf "$prefix"
cmake --install "$ref/build" --prefix "$prefix" > /dev/null

# The configuration of GMT's own CI (ci/config-gmt-unix.sh with RUN_TESTS=true, run by
# .github/workflows/tests.yml), which the baselines and the tests are kept green against: threads
# and OpenMP, the long options under test, the frozen test data of the `static` data server, the
# examples, and enable_testing() (GMT never calls it itself). One difference: a Release build,
# where the CI builds Debug with --coverage. Built Debug here, psxy writes the colour of its x and
# + symbols from memory it never set (`1.75e-152 9.06e+223 2.72e-109 C` in comparesymbols.ps,
# vec_choice.ps, plot_symbols.ps, time_testing_5.ps), which Ghostscript refuses (/limitcheck):
# 4 tests fail on both sides that pass built Release.
cat > "$work/src/cmake/ConfigUser.cmake" <<'CONFIG'
set (CMAKE_BUILD_TYPE Release)
set (GMT_USE_THREADS TRUE)
set (GMT_ENABLE_OPENMP TRUE)
set (CMAKE_C_FLAGS "-Wall -Wdeclaration-after-statement ${CMAKE_C_FLAGS}")
set (CMAKE_C_FLAGS "-Wextra ${CMAKE_C_FLAGS}")
enable_testing()
set (GMT_DATA_SERVER static)
set (DO_EXAMPLES TRUE)
set (DO_TESTS TRUE)
set (DO_API_TESTS ON)
set (DO_SUPPLEMENT_TESTS ON)
set (SUPPORT_EXEC_IN_BINARY_DIR TRUE)
add_definitions(-DUSE_COMMON_LONG_OPTIONS)
add_definitions(-DUSE_MODULE_LONG_OPTIONS)
CONFIG

build=$work/build
rm -rf "$build"
cmake -S "$work/src" -B "$build" \
  -DCMAKE_PREFIX_PATH="$prefix${GMT_DEPS_PREFIX:+;$GMT_DEPS_PREFIX}" \
  -DGSHHG_ROOT="$work/data/gshhg-gmt-$GSHHG_VERSION" -DDCW_ROOT="$work/data/dcw-gmt-$DCW_VERSION" \
  -DCMAKE_EXE_LINKER_FLAGS=-Wl,--enable-new-dtags -DCMAKE_SHARED_LINKER_FLAGS=-Wl,--enable-new-dtags \
  -DCMAKE_MODULE_LINKER_FLAGS=-Wl,--enable-new-dtags > "$work/cmake.log" 2>&1
for feature in 'FFTW library *: /' 'Regex support *: PCRE' 'LAPACK library *: yes' \
  'OpenMP support *: enabled' 'GLIB GTHREAD support *: enabled' 'Found GMT data server *: static'; do
  grep -qE "$feature" "$work/cmake.log" || { echo "cmake: '$feature' not in $work/cmake.log" >&2; exit 1; }
done
cmake --build "$build" --parallel "$(nproc 2>/dev/null || echo 4)" > "$work/make.log" 2>&1
rm -rf "$work/install"
cmake --install "$build" --prefix "$work/install" > "$work/install.log" 2>&1

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
# RUNNER_OS: gmtest reads it, as on GitHub Actions, to take the GMT_KNOWN_FAILURE_LINUX marks of
# the scripts (met_uv.sh...) as known failures, as GMT's CI does.
export PATH="$work/install/bin:$prefix/bin:$PATH" GMT_END_SHOW=off RUNNER_OS=Linux
[ "$(command -v fzmodeler)" = "$work/install/bin/fzmodeler" ] || { echo "fzmodeler of GMT not on PATH" >&2; exit 1; }

# The remote datasets of GMT's ci-caches.yml (the cache its CI tests start from), downloaded from
# the static data server into a seed home directory, three times over as ci-caches.yml does.
# Downloading goes through GDAL (JPEG 2000 tiles converted to netCDF), with the libproj of the
# reference; the passes then start from copies of the seed.
seed=$work/home-seed
rm -rf "$seed" && mkdir -p "$seed"
datasets=$(sed -n '/data="@/,/"$/p' "$work/src/.github/workflows/ci-caches.yml" | grep -oE '@[A-Za-z0-9_.]+')
[ "$(wc -w <<< "$datasets")" -ge 30 ] || { echo "ci-caches.yml: only $(wc -w <<< "$datasets") datasets" >&2; exit 1; }
[ "$(HOME=$seed "$build/src/gmt" --show-dataserver)" = "http://static.generic-mapping-tools.org" ] \
  || { echo "the static data server is not used" >&2; exit 1; }
for _ in 1 2 3; do
  (cd "$seed" && HOME=$seed LD_LIBRARY_PATH="$prefix/lib" "$build/src/gmt" which -Ga $datasets > /dev/null 2>> "$work/seed.log") || true
  # With a ghost server (static), gmtremote_init_paths moves the user directory to
  # ~/.gmt/static, where `gmt which` writes the hash table, but `gmt get` reads it under
  # ~/.gmt/static/server ("Unable to access or read .../server/gmt_hash_server.txt"): GMT 6.5.0
  # gets nothing then. The table is given at the place `gmt get` reads.
  [ -f "$seed/.gmt/static/gmt_hash_server.txt" ] && mkdir -p "$seed/.gmt/static/server" \
    && cp "$seed/.gmt/static/gmt_hash_server.txt" "$seed/.gmt/static/server/"
  (cd "$seed" && HOME=$seed LD_LIBRARY_PATH="$prefix/lib" "$build/src/gmt" get -Dcache >> "$work/seed.log" 2>&1) || true
done
cached=$(find "$seed/.gmt/static/cache" -type f | wc -l)
[ "$cached" -ge 100 ] || { echo "gmt get -Dcache fetched $cached files (see $work/seed.log)" >&2; exit 1; }
missing=0
for d in $datasets; do
  (cd "$seed" && HOME=$seed LD_LIBRARY_PATH="$prefix/lib" "$build/src/gmt" which $d > /dev/null 2>&1) \
    || { echo "dataset not downloaded: $d" >&2; missing=$((missing + 1)); }
done
[ "$missing" = 0 ] || exit 1
echo "seed home: $(wc -w <<< "$datasets") datasets of ci-caches.yml and $cached cache files, $(du -sh "$seed/.gmt" | cut -f1)"
export GMT_HOME=$work/home GMT_HOME_SEED=$seed

# The environment of a pass: its libproj first, the interposer, and its own home directory.
pass_env() {
  local name=$1 libdir=$2 log=$3
  echo "LD_LIBRARY_PATH=$libdir${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
  echo "HOME=$GMT_HOME"
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
  # Each pass starts from a copy of the seed home, at the same path (it is written in the
  # history of the grids): what a test downloads beyond the seed, a pass downloads itself.
  rm -rf "$GMT_HOME"
  cp -a "$GMT_HOME_SEED" "$GMT_HOME"
  [ "$(HOME=$GMT_HOME "$build/src/gmt" --show-userdir)" = "$GMT_HOME/.gmt" ] \
    || { echo "$name: gmt does not use $GMT_HOME/.gmt" >&2; exit 1; }
  check_libs "$name" "$libdir"
  rm -rf "$build/Testing" "$build/test/fail_count.txt" "$log"
  local t0
  t0=$(date +%s)
  (cd "$build" && env $(pass_env "$name" "$libdir" "$log") PATH="$PATH" PROJ_DATA="$PROJ_DATA" \
    PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES ctest --output-on-failure --force-new-ctest-process \
    -j "$(nproc 2>/dev/null || echo 4)" --timeout 480 -O "$work/ctest-$name-1.log" > "$work/ctest-$name-run.log" 2>&1 || true)
  # GMT's CI reruns the failed tests twice (`ctest || ctest --rerun-failed || ctest --rerun-failed`):
  # some scripts race with themselves under a loaded parallel run (a `gmt ... | gmt psxy -R` that
  # reads the history before it is written). The outcome of a test is its last run; the tests a
  # rerun changed are listed by gmt_compare.py.
  cp "$work/ctest-$name-1.log" "$work/ctest-$name.log"
  for rerun in 2 3; do
    grep -qE '\*\*\*(Failed|Timeout|Exception)|Not Run' "$work/ctest-$name-$((rerun - 1)).log" || break
    (cd "$build" && env $(pass_env "$name" "$libdir" "$log") PATH="$PATH" PROJ_DATA="$PROJ_DATA" \
      PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES ctest --output-on-failure --force-new-ctest-process \
      -j "$(nproc 2>/dev/null || echo 4)" --timeout 480 --rerun-failed -O "$work/ctest-$name-$rerun.log" \
      >> "$work/ctest-$name-run.log" 2>&1 || true)
    cat "$work/ctest-$name-$rerun.log" >> "$work/ctest-$name.log"
  done
  cp "$build/test/fail_count.txt" "$work/fail_count-$name.txt" 2>/dev/null || : > "$work/fail_count-$name.txt"
  echo "$name: ctest $(( $(date +%s) - t0 )) s, $(grep -hE 'tests passed' "$work"/ctest-"$name"-[123].log | paste -sd';' || echo 'no summary')"
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
