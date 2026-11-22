#!/usr/bin/env bash
# Run the test suite of libspatialite, a real client of libproj, against PROJ 9.9.0 and against
# the C library of proj-rs, and compare the log of every test (docs/CLIENTS-spatialite.md).
#
# Usage: tools/clients/spatialite.sh [WORKDIR]      (default: target/clients/spatialite)
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), a C
# compiler, network access to gaia-gis.it, and the libraries libspatialite links (Debian:
# libgeos-dev libfreexl-dev libxml2-dev zlib1g-dev librttopo-dev libminizip-dev libsqlite3-dev).
# libspatialite is built once against PROJ 9.9.0; the second run swaps in the libproj of proj-rs
# through LD_LIBRARY_PATH (same soname, RUNPATH). Both runs are offline and read the same data.
set -euo pipefail

VERSION=5.1.0
SHA256=43be2dd349daffe016dd1400c5d11285828c22fea35ca5109f21f3ed50605080
root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/spatialite}
mkdir -p "$work"
work=$(cd "$work" && pwd)

[ -x "$ref/build/bin/proj" ] || { echo "no reference PROJ in $ref: run tools/build-reference.sh" >&2; exit 1; }

tarball=$work/libspatialite-$VERSION.tar.gz
[ -s "$tarball" ] || curl -fsSL \
  "https://www.gaia-gis.it/gaia-sins/libspatialite-sources/libspatialite-$VERSION.tar.gz" -o "$tarball"
echo "$SHA256  $tarball" | sha256sum --check --quiet
rm -rf "$work/src" && mkdir -p "$work/src" && tar -xzf "$tarball" -C "$work/src"
src=$work/src/libspatialite-$VERSION

includes="-I$ref/src/src -I$ref/build/src"
(cd "$src" && ./configure --disable-static CFLAGS="-O1 $includes" CPPFLAGS="$includes" \
  LDFLAGS="-L$ref/build/lib -Wl,-rpath,$ref/build/lib" > "$work/configure.log" 2>&1)
grep -q "define PROJ_NEW 1" "$src/src/headers/spatialite/gaiaconfig.h" \
  || { echo "libspatialite did not find the PROJ 6+ API" >&2; exit 1; }
make -C "$src" -j"$(nproc)" > "$work/make.log" 2>&1

"$root/capi/tests/dist.sh" "$work/ours" > /dev/null
export PROJ_DATA=$ref/build/data PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES PROJ_NETWORK=OFF

run() {
  find "$src/test" \( -name '*.log' -o -name '*.trs' \) -delete
  make -C "$src" -k -j"$(nproc)" check > "$work/check-$1.log" 2>&1 || true
  rm -rf "$work/logs-$1" && mkdir -p "$work/logs-$1"
  (cd "$src" && find test -name '*.log' ! -name test-suite.log -exec cp --parents {} "$work/logs-$1/" \;)
  echo "$1: $(grep -E '^# (TOTAL|PASS|FAIL|ERROR)' "$work/check-$1.log" \
    | awk '{s[$2]+=$3} END {printf "%d passed, %d failed, %d errors of %d", s["PASS:"], s["FAIL:"], s["ERROR:"], s["TOTAL:"]}')"
}
run ref
LD_LIBRARY_PATH=$work/ours/lib run ours

if diff -r "$work/logs-ref" "$work/logs-ours" > "$work/logs.diff"; then
  echo "the $(find "$work/logs-ref" -name '*.log' | wc -l) test logs are identical"
else
  echo "logs differ: $work/logs.diff" >&2
  exit 1
fi
