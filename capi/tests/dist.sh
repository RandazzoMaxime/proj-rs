#!/usr/bin/env bash
# Build the C library of proj-rs and install it under the names of libproj 9.9.0.
#
# Usage: capi/tests/dist.sh [--release] [PREFIX]       (default prefix: target/capi-dist)
#
# Installs PREFIX/lib/libproj.so.25, libproj.so, libproj.a (libproj.25.dylib, libproj.dylib
# and libproj.a on macOS) and PREFIX/include/{proj.h,proj_constants.h,proj_experimental.h,
# proj_symbol_rename.h}. Windows (proj.dll, proj.lib): capi/tests/dist.ps1.
set -euo pipefail

profile=debug
cargo_args=()
if [ "${1:-}" = "--release" ]; then
  profile=release
  cargo_args=(--release)
  shift
fi
root=$(cd "$(dirname "$0")/../.." && pwd)
prefix=${1:-$root/target/capi-dist}
mkdir -p "$prefix/lib" "$prefix/include"

cd "$root"
cargo build -p proj-rs-capi ${cargo_args[@]+"${cargo_args[@]}"}
target=${CARGO_TARGET_DIR:-$root/target}/$profile

case "$(uname -s)" in
  Darwin)
    cp "$target/libproj_rs_capi.dylib" "$prefix/lib/libproj.25.dylib"
    ln -sf libproj.25.dylib "$prefix/lib/libproj.dylib"
    ;;
  *)
    cp "$target/libproj_rs_capi.so" "$prefix/lib/libproj.so.25"
    ln -sf libproj.so.25 "$prefix/lib/libproj.so"
    ;;
esac
cp "$target/libproj_rs_capi.a" "$prefix/lib/libproj.a"
cp "$root"/capi/include/*.h "$prefix/include/"
echo "installed in $prefix"
