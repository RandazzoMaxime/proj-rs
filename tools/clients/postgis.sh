#!/usr/bin/env bash
# Run the regression suite of PostGIS, a real client of libproj loaded by the PostgreSQL server,
# against PROJ 9.9.0 and against the C library of proj-rs (docs/CLIENTS-postgis.md).
#
# Usage (as root): tools/clients/postgis.sh [WORKDIR]      (default: target/clients/postgis)
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), cmake, a C
# compiler, PostgreSQL 16 with its server headers, GEOS, libxml2, json-c (Debian: postgresql-16
# postgresql-server-dev-16 libgeos-dev libxml2-dev libjson-c-dev libfile-which-perl), network
# access to download.osgeo.org. PostGIS is built once against PROJ 9.9.0 and installed into the
# PostgreSQL of the system; a temporary cluster then runs the suite twice, its server started
# with LD_LIBRARY_PATH on PROJ 9.9.0, then on the libproj of proj-rs (same soname, RUNPATH).
# Raster is left out (--without-raster): it reaches libproj through GDAL.
set -euo pipefail

VERSION=3.6.4
SHA256=ed8dc6679f1e06f7b113592b04cde2a7e00f1b1e681294c8ca2204058990cec6
root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/postgis}
mkdir -p "$work"
work=$(cd "$work" && pwd)
pgbin=$(pg_config --bindir)

[ -x "$ref/build/bin/proj" ] || { echo "no reference PROJ in $ref: run tools/build-reference.sh" >&2; exit 1; }
[ "$(id -u)" = 0 ] || { echo "run as root: PostGIS is installed into the PostgreSQL of the system" >&2; exit 1; }

tarball=$work/postgis-$VERSION.tar.gz
[ -s "$tarball" ] || curl -fsSL "https://download.osgeo.org/postgis/source/postgis-$VERSION.tar.gz" -o "$tarball"
echo "$SHA256  $tarball" | sha256sum --check --quiet
rm -rf "$work/src" && mkdir -p "$work/src" && tar -xzf "$tarball" -C "$work/src"
src=$work/src/postgis-$VERSION

prefix=$work/ref-prefix
rm -rf "$prefix"
cmake --install "$ref/build" --prefix "$prefix" > /dev/null
(cd "$src" && ./configure --with-projdir="$prefix" --without-raster --without-protobuf \
  LDFLAGS="-Wl,--enable-new-dtags -Wl,-rpath,$prefix/lib" > "$work/configure.log" 2>&1)
make -C "$src" -j"$(nproc)" > "$work/make.log" 2>&1
make -C "$src" install > "$work/install.log" 2>&1
"$root/capi/tests/dist.sh" "$work/ours" > /dev/null

cluster=$(mktemp -d /var/tmp/postgis-regress.XXXXXX)
mkdir -p "$cluster/sock" && chown -R postgres:postgres "$cluster"
su postgres -c "$pgbin/initdb -D $cluster/data --auth=trust -E UTF8 --locale=C" > "$work/initdb.log"
data_env="PROJ_DATA=$prefix/share/proj PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES PROJ_NETWORK=OFF"
server() {
  su postgres -c "env LD_LIBRARY_PATH=$2 $data_env $pgbin/pg_ctl -D $cluster/data \
    -o '-k $cluster/sock -p 5439 -c listen_addresses=' -l $cluster/server.log -w $1" > /dev/null
}
export PGHOST=$cluster/sock PGPORT=5439 PGUSER=postgres
export PROJ_DATA=$prefix/share/proj PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES PROJ_NETWORK=OFF
run() {
  server start "$2"
  make -C "$src" check-regress > "$work/regress-$1.log" 2>&1 || true
  server stop "$2"
  grep -E ' \.\. failed' "$work/regress-$1.log" | sed 's/ in [0-9]* ms//' > "$work/failed-$1.txt" || true
  echo "$1: $(grep -E '^Run tests' "$work/regress-$1.log" | tail -1), $(grep -E '^Failed' "$work/regress-$1.log" | tail -1)"
}
run ref "$prefix/lib"
run ours "$work/ours/lib"
rm -rf "$cluster"

diff "$work/failed-ref.txt" "$work/failed-ours.txt" || { echo "the failing tests differ" >&2; exit 1; }
echo "the same tests fail with both libraries: $(wc -l < "$work/failed-ref.txt")"
