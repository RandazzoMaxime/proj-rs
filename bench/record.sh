#!/usr/bin/env sh
# Runs the progress bench and appends one line to bench/history.jsonl, then redraws the
# charts of bench/ and rewrites the README numbers from the whole history.
# Compare only lines with the same "machine". Commit history.jsonl and the charts with the lot.
#
# BENCH_MACHINE  machine name of the line (default: hostname)
# BENCH_ROUNDS   runs of the bench; each case keeps its smallest time (default: 1)
# PROJ_DATA      directory with proj.db and tests/ (default: data/)
#
# For the libproj columns, build with PROJ_RS_LIBPROJ_DIR set to a Release libproj 9.9.0,
# and pin the process, for example `taskset -c 2 sh bench/record.sh`.
set -eu
cd "$(dirname "$0")/.."
export PROJ_DATA="${PROJ_DATA:-$PWD/data}"
export PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY="${PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY:-YES}"
rounds="${BENCH_ROUNDS:-1}"
results=$(mktemp)
trap 'rm -f "$results"' EXIT
cargo build --quiet --release --bin proj-rs-bench
round=0
while [ "$round" -lt "$rounds" ]; do
  cargo run --quiet --release --bin proj-rs-bench >> "$results"
  round=$((round + 1))
done
if [ "$rounds" -gt 1 ]; then
  result=$(python3 tools/bench/min_of_runs.py < "$results")
else
  result=$(cat "$results")
fi
commit=$(git rev-parse --short HEAD)
dirty=$(git diff --quiet HEAD -- src && echo false || echo true)
machine="${BENCH_MACHINE:-$(hostname)}"
date=$(date -u +%Y-%m-%dT%H:%M:%SZ)
passes=$(awk '$1 == "network" { print $2 }' tests/gie/passing.txt)
line="{\"date\":\"$date\",\"commit\":\"$commit\",\"dirty\":$dirty,\"machine\":\"$machine\",\"passes\":$passes,\"result\":$result}"
echo "$line" >> bench/history.jsonl
echo "$line"
python3 tools/bench/trend_svg.py
python3 tools/bench/how_fast.py
