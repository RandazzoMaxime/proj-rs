#!/usr/bin/env bash
# Run the test suite of libspatialite, a real client of libproj, against PROJ 9.9.0 and against
# the C library of proj-rs, and compare the log of every test (docs/CLIENTS-spatialite.md).
#
# Usage: tools/clients/spatialite.sh [WORKDIR]      (default: target/clients/spatialite)
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), a C
# compiler, network access to gaia-gis.it and sqlite.org, and the libraries libspatialite links
# (Debian: libgeos-dev libfreexl-dev libxml2-dev zlib1g-dev librttopo-dev libminizip-dev,
# installed by the script when it runs as root on Debian or Ubuntu; SQLite is built here).
# libspatialite is built once against PROJ 9.9.0, with its test programs; the second run swaps in
# the libproj of proj-rs through LD_LIBRARY_PATH (same soname, RUNPATH). Both runs are offline and read the same data.
# Which libproj each pass loads is proven by libproj_proof.sh (ldd, no DT_RPATH, LD_DEBUG=libs,
# the proj_count.c interposer for the whole `make check`).
set -euo pipefail

VERSION=5.1.0
SHA256=43be2dd349daffe016dd1400c5d11285828c22fea35ca5109f21f3ed50605080
# libspatialite 5.1.0 (2023-08-04) is built against SQLite 3.42.0, the release its test suite was
# written for. From SQLite 3.43.0, printf('%1.0f', 1e128) and printf('%.300f', 1e128) no longer
# give the same digits, and check_bufovflw (AsKML#1, which builds its expected string with the
# first and gets the second from AsKml) fails whatever the libproj.
SQLITE_VERSION=3420000
SQLITE_YEAR=2023
SQLITE_SHA256=1cc824d0f5e675829fa37018318fda833ea56f7e9de2b41eddd9f7643b5ec29e
# Test processes of a pass that call one of the functions proj_count.c counts (7 of them; all 108
# test programs map libproj).
MIN_CALLERS=6
root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/spatialite}
mkdir -p "$work"
work=$(cd "$work" && pwd)
. "$root/tools/clients/libproj_proof.sh"

[ -x "$ref/build/bin/proj" ] || { echo "no reference PROJ in $ref: run tools/build-reference.sh" >&2; exit 1; }

packages=(libgeos-dev libfreexl-dev libxml2-dev zlib1g-dev librttopo-dev libminizip-dev unzip
  pkg-config)
if [ "$(id -u)" = 0 ] && command -v apt-get > /dev/null; then
  missing=()
  for p in "${packages[@]}"; do dpkg -s "$p" > /dev/null 2>&1 || missing+=("$p"); done
  [ ${#missing[@]} = 0 ] || { apt-get update -q > /dev/null; apt-get install -y -q --no-install-recommends "${missing[@]}" > /dev/null; }
fi

tarball=$work/libspatialite-$VERSION.tar.gz
[ -s "$tarball" ] || curl -fsSL \
  "https://www.gaia-gis.it/gaia-sins/libspatialite-sources/libspatialite-$VERSION.tar.gz" -o "$tarball"
echo "$SHA256  $tarball" | sha256sum --check --quiet
rm -rf "$work/src" && mkdir -p "$work/src" && tar -xzf "$tarball" -C "$work/src"
src=$work/src/libspatialite-$VERSION

# SQLite, as a shared library with the options of Debian's build that libspatialite uses.
sqzip=$work/sqlite-amalgamation-$SQLITE_VERSION.zip
[ -s "$sqzip" ] || curl -fsSL "https://www.sqlite.org/$SQLITE_YEAR/sqlite-amalgamation-$SQLITE_VERSION.zip" -o "$sqzip"
echo "$SQLITE_SHA256  $sqzip" | sha256sum --check --quiet
sq=$work/sqlite
rm -rf "$sq" "$work/sqlite-amalgamation-$SQLITE_VERSION"
unzip -q "$sqzip" -d "$work"
mkdir -p "$sq/include" "$sq/lib/pkgconfig" "$sq/bin"
amal=$work/sqlite-amalgamation-$SQLITE_VERSION
cp "$amal/sqlite3.h" "$amal/sqlite3ext.h" "$sq/include/"
sqopts="-DSQLITE_ENABLE_COLUMN_METADATA -DSQLITE_ENABLE_RTREE -DSQLITE_ENABLE_FTS3
  -DSQLITE_ENABLE_FTS3_PARENTHESIS -DSQLITE_ENABLE_FTS4 -DSQLITE_ENABLE_FTS5
  -DSQLITE_ENABLE_MATH_FUNCTIONS -DSQLITE_ENABLE_DBSTAT_VTAB -DSQLITE_ENABLE_LOAD_EXTENSION
  -DSQLITE_SOUNDEX -DSQLITE_SECURE_DELETE -DSQLITE_USE_URI -DSQLITE_MAX_VARIABLE_NUMBER=250000"
cc -O2 -fPIC -shared $sqopts -Wl,-soname,libsqlite3.so.0 "$amal/sqlite3.c" -o "$sq/lib/libsqlite3.so.0" -lpthread -ldl -lm
ln -sf libsqlite3.so.0 "$sq/lib/libsqlite3.so"
cc -O2 $sqopts -I"$sq/include" "$amal/shell.c" -o "$sq/bin/sqlite3" -L"$sq/lib" -lsqlite3 \
  -Wl,--enable-new-dtags -Wl,-rpath,"$sq/lib" -lpthread -ldl -lm
printf 'prefix=%s\nlibdir=${prefix}/lib\nincludedir=${prefix}/include\nName: SQLite\nDescription: SQL database engine\nVersion: %s\nLibs: -L${libdir} -lsqlite3\nCflags: -I${includedir}\n' \
  "$sq" "$("$sq/bin/sqlite3" :memory: 'select sqlite_version()')" > "$sq/lib/pkgconfig/sqlite3.pc"
echo "SQLite $("$sq/bin/sqlite3" :memory: 'select sqlite_version()')"

includes="-I$sq/include -I$ref/src/src -I$ref/build/src"
(cd "$src" && PKG_CONFIG_PATH="$sq/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}" ./configure \
  --disable-static CFLAGS="-O1 $includes" CPPFLAGS="$includes" \
  LDFLAGS="-L$sq/lib -L$ref/build/lib -Wl,--enable-new-dtags -Wl,-rpath,$sq/lib:$ref/build/lib" \
  > "$work/configure.log" 2>&1)
grep -q "define PROJ_NEW 1" "$src/src/headers/spatialite/gaiaconfig.h" \
  || { echo "libspatialite did not find the PROJ 6+ API" >&2; exit 1; }
make -C "$src" -j"$(nproc)" > "$work/make.log" 2>&1
got=$(ldd "$src/src/.libs/libspatialite.so" | awk '$1 == "libsqlite3.so.0" { print $3 }')
[ "$(readlink -f "$got")" = "$(readlink -f "$sq/lib/libsqlite3.so.0")" ] \
  || { echo "libspatialite resolves libsqlite3.so.0 to '$got', not $sq/lib" >&2; exit 1; }
# The test programs are built here, once: both passes run the same binaries.
make -C "$src" -j"$(nproc)" check TESTS= > "$work/make-tests.log" 2>&1

"$root/capi/tests/dist.sh" "$work/ours" > /dev/null
proof_build "$work" "$ref/src/src"
proof_distinct "$ref/build/lib" "$work/ours/lib"
export PROJ_DATA=$ref/build/data PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES PROJ_NETWORK=OFF

lib=$src/src/.libs
run() {
  local name=$1 libdir=$2 log=$work/projcount-$1.log
  rm -f "$log"
  (cd "$work" && proof_static "$name" "$libdir" "$libdir" "$lib/libspatialite.so" "$lib/mod_spatialite.so")
  (cd "$work" && proof_call "$name" "$libdir" "$libdir" "$sq/bin/sqlite3" :memory: \
    "SELECT load_extension('$lib/mod_spatialite'); SELECT InitSpatialMetadata(1);
     SELECT AsText(ST_Transform(MakePoint(3, 45, 4326), 32631));")
  find "$src/test" \( -name '*.log' -o -name '*.trs' \) -delete
  env $(proof_env "$libdir" "$log") make -C "$src" -k -j"$(nproc)" check > "$work/check-$name.log" 2>&1 || true
  # check_sql_stmt, _tiny and _extension stop at their first failing case: transform4 of
  # sql_stmt_proj910_tests expects the message of PROJ 9.1 to 9.4, and PROJ 9.5 added the code
  # (": NONE:0"). They run once more with that message, so the cases after it run too.
  local case=$src/test/sql_stmt_proj910_tests/transform4.testcase t
  cp "$case" "$work/transform4.testcase.orig"
  sed -i 's/PROJ reports "proj_create: crs not found"\.$/PROJ reports "proj_create: crs not found: NONE:0"./' "$case"
  grep -q 'crs not found: NONE:0' "$case" || { echo "transform4.testcase: patch did not apply" >&2; exit 1; }
  rm -rf "$work/rerun-$name" && mkdir -p "$work/rerun-$name"
  for t in check_sql_stmt check_sql_stmt_tiny check_sql_stmt_extension; do
    (cd "$src/test" && env $(proof_env "$libdir" "$log") "./$t" > "$work/rerun-$name/$t.log" 2>&1 \
      && echo "$name: $t with the message of PROJ 9.5: passed, $(grep -c '^Test case:' "$work/rerun-$name/$t.log") cases" \
      || echo "$name: $t with the message of PROJ 9.5: FAILED, $(grep -c '^Test case:' "$work/rerun-$name/$t.log") cases")
  done
  cp "$work/transform4.testcase.orig" "$case"
  proof_log "$name" "$libdir" "$log" "$MIN_CALLERS"
  rm -rf "$work/logs-$name" && mkdir -p "$work/logs-$name"
  (cd "$src" && find test -name '*.log' ! -name test-suite.log -exec cp --parents {} "$work/logs-$name/" \;)
  echo "$name: $(grep -E '^# (TOTAL|PASS|FAIL|ERROR)' "$work/check-$name.log" \
    | awk '{s[$2]+=$3} END {printf "%d passed, %d failed, %d errors of %d", s["PASS:"], s["FAIL:"], s["ERROR:"], s["TOTAL:"]}')"
  grep -hE '^(FAIL|ERROR):' "$work/check-$name.log" | sed "s/^/$name: /" || true
}
run ref "$ref/build/lib"
run ours "$work/ours/lib"

diff -r "$work/rerun-ref" "$work/rerun-ours" > "$work/rerun.diff" \
  || { echo "the reruns with the message of PROJ 9.5 differ: $work/rerun.diff" >&2; exit 1; }
if diff -r "$work/logs-ref" "$work/logs-ours" > "$work/logs.diff"; then
  echo "the $(find "$work/logs-ref" -name '*.log' | wc -l) test logs are identical"
else
  echo "logs differ: $work/logs.diff" >&2
  exit 1
fi
