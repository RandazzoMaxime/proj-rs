#!/usr/bin/env sh
# Builds the programs of bench/footprint/, measures the size on disk and the peak memory of
# proj-rs against PROJ 9.9.0, writes bench/footprint.json, then redraws bench/{size,memory}-*.svg
# and the README block. Linux x86-64 only. Needs cmake, a C and C++ compiler, the libtiff, libcurl
# and SQLite development headers, valgrind (optional: without it, no heap column), strip, ldd.
#
# BENCH_MACHINE  machine name written in the file (default: hostname)
# PROJ_REF       dir of tools/build-reference.sh (default: target/proj-reference), built if missing
# FOOTPRINT_OUT  build dir of this script (default: target/footprint)
# FOOTPRINT_JOBS parallel jobs of the builds (default: 2)
#
# What is measured and how: docs/BENCHMARK.md, section "Size and memory".
set -eu
cd "$(dirname "$0")/.."
root=$PWD
ref=${PROJ_REF:-$root/target/proj-reference}
out=${FOOTPRINT_OUT:-$root/target/footprint}
jobs=${FOOTPRINT_JOBS:-2}
mkdir -p "$out"

[ -f "$ref/build/lib/libproj.so" ] || tools/build-reference.sh "$ref"
full=$out/proj-full
if [ ! -f "$full/build/lib/libproj.so" ]; then
  mkdir -p "$full"
  [ -e "$full/src" ] || ln -s "$ref/src" "$full/src"
  PROJ_REF_TIFF_CURL=ON tools/build-reference.sh "$full" > /dev/null
fi

# proj-rs: the Rust programs (own workspace, release profile of the root), the default build
# and the one with the `network` feature (HTTPS download, TLS from the system), and the C library.
env -u PROJ_RS_LIBPROJ_DIR CARGO_TARGET_DIR="$out/cargo" \
  cargo build --quiet --release -j "$jobs" --manifest-path bench/footprint/rs/Cargo.toml
env -u PROJ_RS_LIBPROJ_DIR CARGO_TARGET_DIR="$out/cargo-network" \
  cargo build --quiet --release -j "$jobs" --manifest-path bench/footprint/rs/Cargo.toml \
  --features network --bin footprint-rs
env -u PROJ_RS_LIBPROJ_DIR CARGO_TARGET_DIR="$out/cargo" CARGO_BUILD_JOBS="$jobs" \
  capi/tests/dist.sh --release "$out/capi-dist" > /dev/null

# The same C program against each libproj.
c=bench/footprint/c/convert.c
cc -O2 -o "$out/convert-proj" "$c" -I"$ref/src/src" -L"$ref/build/lib" -lproj -lm \
  -Wl,-rpath,"$ref/build/lib"
cc -O2 -o "$out/convert-proj-full" "$c" -I"$full/src/src" -L"$full/build/lib" -lproj -lm \
  -Wl,-rpath,"$full/build/lib"
cc -O2 -o "$out/convert-capi" "$c" -I"$out/capi-dist/include" -L"$out/capi-dist/lib" -lproj -lm \
  -Wl,-rpath,"$out/capi-dist/lib"

python3 tools/bench/footprint_measure.py \
  --proj-lib "$ref/build/lib" --proj-db "$ref/build/data/proj.db" \
  --proj-full-lib "$full/build/lib" --proj-full-db "$full/build/data/proj.db" \
  --capi-lib "$out/capi-dist/lib/libproj.so.25" --rs-db "$root/data/proj.db" \
  --c-proj "$out/convert-proj" --c-proj-full "$out/convert-proj-full" --c-capi "$out/convert-capi" \
  --rs-bin "$out/cargo/release/footprint-rs" --rs-network-bin "$out/cargo-network/release/footprint-rs" \
  --rs-empty "$out/cargo/release/empty-rs" \
  --machine "${BENCH_MACHINE:-$(hostname)}" --out bench/footprint.json
python3 tools/bench/footprint_block.py
python3 tools/bench/cards.py
