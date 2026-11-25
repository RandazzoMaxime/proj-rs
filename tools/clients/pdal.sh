#!/usr/bin/env bash
# Run the test suite of PDAL (point clouds), a real client of libproj both directly and through
# GDAL, against PROJ 9.9.0 and against the C library of proj-rs (docs/CLIENTS-pdal.md).
#
# Usage: tools/clients/pdal.sh [WORKDIR]      (default: target/clients/pdal)
#
# Needs: tools/build-reference.sh done (PROJ_REFERENCE, default target/proj-reference), cmake, a
# C++ compiler, GDAL 3.8 or later with its CMake package and libcurl (Debian: libgdal-dev
# libcurl4-openssl-dev, installed by the script when it runs as root on Debian or Ubuntu), network
# access to github.com. The source is pinned by its commit. PDAL is built once against PROJ
# 9.9.0; the second run swaps in the libproj of proj-rs through LD_LIBRARY_PATH, for PDAL and for
# GDAL alike (same soname). The tests run one at a time: some of them share temporary files.
#
# Two tests read remote files: pdal_io_stac_reader_test (s3-us-west-2.amazonaws.com, and the
# JSON schemas of json-schema.org) and pdal_io_copc_remote_reader_test
# (github.com/PDAL/data/raw/..., directly and through GDAL's /vsicurl/). The script checks first
# that those URLs answer, and says so when one does not: a failure of those tests then comes from
# the network, not from libproj. When github.com refuses the COPC file (a proxy that scopes GitHub
# to some repositories), the test reads PDAL's own copy on S3 instead, once its sha256 is checked
# against the Git LFS object id of the GitHub file: the same bytes, through the same code paths. Which libproj each pass loads is proven by libproj_proof.sh
# (ldd, no DT_RPATH, LD_DEBUG=libs, the proj_count.c interposer for the whole ctest pass).
set -euo pipefail

TAG=2.10.2
COMMIT=27008f6241be44585c866a29dfc13bb16d678dab
REMOTE_URLS=(
  https://s3-us-west-2.amazonaws.com/usgs-lidar-stac/ept/MD_GoldenBeach_2012.json
  https://github.com/PDAL/data/raw/refs/heads/main/autzen/autzen-classified.copc.laz
)
COPC_GITHUB=https://github.com/PDAL/data/raw/refs/heads/main/autzen/autzen-classified.copc.laz
COPC_MIRROR=https://s3.amazonaws.com/hobu-lidar/autzen-classified.copc.laz
COPC_LFS_OID=db2d56cdfa058bffccdc5d6019dae2fc9c6a551df10a5523c06c76a3e25a27fa
# Processes of a ctest pass that call one of the functions proj_count.c counts (111 of them).
MIN_CALLERS=100
root=$(cd "$(dirname "$0")/../.." && pwd)
ref=${PROJ_REFERENCE:-$root/target/proj-reference}
work=${1:-$root/target/clients/pdal}
mkdir -p "$work"
work=$(cd "$work" && pwd)
. "$root/tools/clients/libproj_proof.sh"

[ -x "$ref/build/bin/proj" ] || { echo "no reference PROJ in $ref: run tools/build-reference.sh" >&2; exit 1; }

packages=(libgdal-dev libcurl4-openssl-dev libzstd-dev liblzma-dev libxml2-dev zlib1g-dev)
if [ "$(id -u)" = 0 ] && command -v apt-get > /dev/null; then
  missing=()
  for p in "${packages[@]}"; do dpkg -s "$p" > /dev/null 2>&1 || missing+=("$p"); done
  [ ${#missing[@]} = 0 ] || { apt-get update -q > /dev/null; apt-get install -y -q --no-install-recommends "${missing[@]}" > /dev/null; }
fi

# PDAL's HTTP layer (arbiter) gives the proxy of HTTPS_PROXY to http:// URLs too. Behind an
# HTTPS-only proxy (CONNECT only, no HTTP_PROXY), its plain-HTTP requests are refused, and
# pdal_io_stac_reader_test fails on the JSON schema of its catalogs (http://json-schema.org/...).
# split_proxy.py then stands between: CONNECT goes to that proxy unchanged, plain HTTP goes
# straight to its host, as for any client with HTTPS_PROXY and no HTTP_PROXY.
if [ -n "${HTTPS_PROXY:-${https_proxy:-}}" ] && [ -z "${HTTP_PROXY:-${http_proxy:-}}" ]; then
  rm -f "$work/split-proxy.port"
  python3 -I "$root/tools/clients/split_proxy.py" "$work/split-proxy.port" &
  split_pid=$!
  trap 'kill "$split_pid" 2>/dev/null || true' EXIT
  for _ in $(seq 50); do [ -s "$work/split-proxy.port" ] && break; sleep 0.1; done
  export CURL_PROXY=http://127.0.0.1:$(cat "$work/split-proxy.port")
  echo "arbiter proxy: $CURL_PROXY (split_proxy.py: CONNECT to ${HTTPS_PROXY:-$https_proxy}, plain HTTP direct)"
fi

reachable() {
  curl -s -o /dev/null -r 0-99 -w '%{http_code}' ${CURL_PROXY:+-x "$CURL_PROXY"} "$1" || true
}
copc_url=$COPC_GITHUB
code=$(reachable "$COPC_GITHUB")
case "$code" in
  200|206|301|302) ;;
  *)
    echo "github.com refuses the COPC file (HTTP $code); checking PDAL's copy $COPC_MIRROR"
    sum=$(curl -sS --fail ${CURL_PROXY:+-x "$CURL_PROXY"} "$COPC_MIRROR" | sha256sum | cut -d' ' -f1)
    [ "$sum" = "$COPC_LFS_OID" ] || { echo "the copy does not match the LFS object of PDAL/data ($sum)" >&2; exit 1; }
    echo "the copy matches the LFS object of PDAL/data (sha256 $sum): the test reads it"
    copc_url=$COPC_MIRROR ;;
esac
for url in "${REMOTE_URLS[@]/$COPC_GITHUB/$copc_url}" http://json-schema.org/draft-07/schema; do
  code=$(reachable "$url")
  case "$code" in
    200|206|301|302) echo "remote test file reachable: $url" ;;
    *) echo "remote test file NOT reachable (HTTP $code): $url; the test that reads it fails on both sides" ;;
  esac
done

rm -rf "$work/src"
git clone -q --depth 1 --branch "$TAG" https://github.com/PDAL/PDAL.git "$work/src" 2>/dev/null
[ "$(git -C "$work/src" rev-parse HEAD)" = "$COMMIT" ] || { echo "$TAG is not $COMMIT" >&2; exit 1; }
if [ "$copc_url" != "$COPC_GITHUB" ]; then
  copc_test=$work/src/test/unit/io/CopcRemoteReaderTest.cpp
  grep -qF "\"$COPC_GITHUB\"" "$copc_test" || { echo "$copc_test no longer reads $COPC_GITHUB" >&2; exit 1; }
  sed -i "s|$COPC_GITHUB|$copc_url|" "$copc_test"
fi

prefix=$work/ref-prefix
rm -rf "$prefix" "$work/build"
cmake --install "$ref/build" --prefix "$prefix" > /dev/null
cmake -S "$work/src" -B "$work/build" -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH="$prefix" \
  -DPROJ_DIR="$prefix/lib/cmake/proj" -DWITH_TESTS=ON -DBUILD_PLUGIN_PGPOINTCLOUD=OFF \
  -DCMAKE_BUILD_RPATH="$prefix/lib" -DCMAKE_EXE_LINKER_FLAGS=-Wl,--enable-new-dtags \
  -DCMAKE_SHARED_LINKER_FLAGS=-Wl,--enable-new-dtags > "$work/cmake.log" 2>&1
cmake --build "$work/build" -j"$(nproc)" > "$work/build.log" 2>&1
"$root/capi/tests/dist.sh" "$work/ours" > /dev/null
proof_build "$work" "$prefix/include"
proof_distinct "$prefix/lib" "$work/ours/lib"
export PROJ_DATA=$prefix/share/proj PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES PROJ_NETWORK=OFF

bin=$work/build/bin
run() {
  local name=$1 libdir=$2 log=$work/projcount-$1.log
  rm -f "$log"
  (cd "$work" && proof_static "$name" "$libdir" "$libdir" "$bin/pdal" "$work/build/lib/libpdalcpp.so")
  (cd "$work" && proof_call "$name" "$libdir" "$libdir" "$bin/pdal" translate \
    "$work/src/test/data/las/simple.las" "$work/proof-$name.las" -f filters.reprojection \
    --filters.reprojection.in_srs=EPSG:32617 --filters.reprojection.out_srs=EPSG:4326)
  (cd "$work/build" && env $(proof_env "$libdir" "$log") ctest -j1 --timeout 300 \
    > "$work/ctest-$name.log" 2>&1 || true)
  proof_log "$name" "$libdir" "$log" "$MIN_CALLERS"
  grep -E '^[[:space:]]+[0-9]+ - ' "$work/ctest-$name.log" | sed -E 's/^[[:space:]]+//' > "$work/failed-$name.txt" || true
  echo "$name: $(grep -E 'tests passed' "$work/ctest-$name.log")"
  sed "s/^/$name: failed: /" "$work/failed-$name.txt"
}
run ref "$prefix/lib"
run ours "$work/ours/lib"

if diff "$work/failed-ref.txt" "$work/failed-ours.txt"; then
  echo "the same tests fail with both libraries: $(wc -l < "$work/failed-ref.txt")"
else
  echo "the failing tests differ" >&2
  exit 1
fi
