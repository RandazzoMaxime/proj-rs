#!/usr/bin/env sh
# Runs the progress bench and appends one line to bench/history.jsonl.
# Compare only lines with the same "machine". Commit history.jsonl with the lot.
set -eu
cd "$(dirname "$0")/.."
result=$(cargo run --quiet --release --bin proj-rs-bench)
commit=$(git rev-parse --short HEAD)
dirty=$(git diff --quiet HEAD -- src && echo false || echo true)
machine="${BENCH_MACHINE:-$(hostname)}"
date=$(date -u +%Y-%m-%dT%H:%M:%SZ)
line="{\"date\":\"$date\",\"commit\":\"$commit\",\"dirty\":$dirty,\"machine\":\"$machine\",\"result\":$result}"
echo "$line" >> bench/history.jsonl
echo "$line"
