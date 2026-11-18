#!/usr/bin/env sh
# Times a billion points, EPSG:4326 to 2154, with proj-rs and with libproj (if linked), and
# writes bench/billion.json, then redraws the cards. A few minutes; pin the process, for
# example `taskset -c 2 sh bench/billion.sh`. Build with PROJ_RS_LIBPROJ_DIR for the libproj side.
set -eu
cd "$(dirname "$0")/.."
export PROJ_DATA="${PROJ_DATA:-$PWD/data}"
export PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY="${PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY:-YES}"
cargo build --quiet --release --bin proj-rs-bench
result=$(cargo run --quiet --release --bin proj-rs-bench -- --billion)
commit=$(git rev-parse --short HEAD)
machine="${BENCH_MACHINE:-$(hostname)}"
date=$(date -u +%Y-%m-%dT%H:%M:%SZ)
printf '{"date":"%s","commit":"%s","machine":"%s","result":%s}\n' "$date" "$commit" "$machine" "$result" > bench/billion.json
cat bench/billion.json
python3 tools/bench/cards.py
