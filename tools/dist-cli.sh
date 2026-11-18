#!/usr/bin/env bash
# Build the command-line tools of proj-rs as ONE executable and install it under the names of the
# PROJ tools: PREFIX/bin/proj-rs, plus PREFIX/bin/cs2cs, proj, cct, ... as symbolic links to it.
#
# Usage: tools/dist-cli.sh [--release] [PREFIX]        (default prefix: target/cli-dist)
#
# --release builds with the `release-cli` profile of Cargo.toml: the `release` profile plus fat
# LTO and stripped symbols. The C library (capi/tests/dist.sh) keeps the `release` profile.
#
# PROJ_RS_CLI_FEATURES="network" adds `projsync` and the HTTPS grid download (off by default).
#
# Each tool is an 18 MB program with proj.db inside (10.5 MB of it); the eight of them cost 144 MB.
# The one executable costs 18 MB and runs the tool whose name it is started under. The links are
# relative, so the prefix can move. Windows: tools/dist-cli.ps1.
set -euo pipefail

profile=debug
cargo_args=()
if [ "${1:-}" = "--release" ]; then
  profile=release-cli
  cargo_args=(--profile release-cli)
  shift
fi
root=$(cd "$(dirname "$0")/.." && pwd)
prefix=${1:-$root/target/cli-dist}

cd "$root"
cargo build --bin proj-rs ${PROJ_RS_CLI_FEATURES:+--features "$PROJ_RS_CLI_FEATURES"} ${cargo_args[@]+"${cargo_args[@]}"}
target=${CARGO_TARGET_DIR:-$root/target}/$profile

mkdir -p "$prefix/bin"
cp "$target/proj-rs" "$prefix/bin/proj-rs"
# The tool names come from the executable, so they follow the features it was built with.
for tool in $("$prefix/bin/proj-rs" --list); do
  ln -sf proj-rs "$prefix/bin/$tool"
done
echo "installed in $prefix/bin"
