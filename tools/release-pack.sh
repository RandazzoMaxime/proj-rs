#!/usr/bin/env bash
# Build and pack the two archives of one platform (Linux, macOS). The release workflow runs it on
# every platform; Windows: tools/release-pack.ps1.
#
# Usage: tools/release-pack.sh <target> [OUTDIR]      (default OUTDIR: target/release-archives)
#
#   <target>  the platform name of the archives: linux-x86_64, macos-arm64, macos-x86_64.
#
# Makes, with <version> read from Cargo.toml:
#   OUTDIR/proj-rs-<version>-<target>-cli.tar.gz       bin/proj-rs and one link per tool name
#   OUTDIR/proj-rs-<version>-<target>-libproj.tar.gz   lib/libproj.so.25 (or .25.dylib), libproj.so
#                                                      (or .dylib), libproj.a, include/*.h,
#                                                      lib/pkgconfig/proj.pc, lib/cmake/proj/*.cmake,
#                                                      share/proj/proj.db
# Each holds LICENSE, NOTICE.md (with the licence texts it points to) and a README.txt.
#
# Run tools/release-smoke.sh on the archives afterwards.
set -euo pipefail

target_name=${1:?usage: tools/release-pack.sh <target> [OUTDIR]}
root=$(cd "$(dirname "$0")/.." && pwd)
out=${2:-$root/target/release-archives}
version=$(grep -m1 '^version = ' "$root/Cargo.toml" | cut -d'"' -f2)
# The PROJ version whose proj.h the library implements: what pkg-config and CMake report.
proj_part() { sed -n "s/^#define PROJ_VERSION_$1 \([0-9]*\)$/\1/p" "$root/capi/include/proj.h"; }
proj_major=$(proj_part MAJOR) proj_minor=$(proj_part MINOR) proj_patch=$(proj_part PATCH)
proj_version=$proj_major.$proj_minor.$proj_patch
[ -n "$proj_major" ] && [ -n "$proj_minor" ] && [ -n "$proj_patch" ] || { echo "no PROJ_VERSION_* in proj.h" >&2; exit 1; }
mkdir -p "$out"
out=$(cd "$out" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cli_dir=proj-rs-$version-$target_name-cli
lib_dir=proj-rs-$version-$target_name-libproj

"$root/tools/dist-cli.sh" --release "$work/cli-prefix"
"$root/capi/tests/dist.sh" --release "$work/lib-prefix"

# `libproj.a` needs the system libraries Rust's std uses: rustc prints them for the staticlib.
# The crate is rebuilt with the same profile; only its own code is compiled again.
# `--color never`: with CARGO_TERM_COLOR=always (CI) the line ends in an escape code, `-lc\e[0m`.
static_libs=$(cd "$root" && cargo --color never rustc --release -p proj-rs-capi --lib --crate-type staticlib -- --print native-static-libs 2>&1 \
  | sed -n 's/^.*native-static-libs: //p' | tail -1)
[ -n "$static_libs" ] || { echo "rustc printed no native-static-libs" >&2; exit 1; }
case $static_libs in *"$(printf '\033')"*) echo "native-static-libs has an escape code: $static_libs" >&2; exit 1 ;; esac

case "$(uname -s)" in
  Darwin)
    lib_files="libproj.25.dylib, libproj.dylib (a link), libproj.a"
    soname=libproj.25.dylib
    imported_key=IMPORTED_SONAME_RELEASE
    imported_value=@rpath/libproj.25.dylib
    ldconfig=""
    userdir='~/Library/Application Support/proj'
    ;;
  *)
    lib_files="libproj.so.25, libproj.so (a link), libproj.a"
    soname=libproj.so.25
    imported_key=IMPORTED_SONAME_RELEASE
    imported_value=libproj.so.25
    ldconfig=" \\&\\& ldconfig"  # `&` is a match in a sed replacement
    userdir='$XDG_DATA_HOME/proj or ~/.local/share/proj'
    ;;
esac

licences() {
  mkdir -p "$1/data/licenses"
  cp "$root/LICENSE" "$root/NOTICE.md" "$1/"
  cp "$root/data/COPYING" "$1/data/"
  cp "$root"/data/licenses/* "$1/data/licenses/"
}

fill() { # fill <template> <destination>
  sed -e "s|@VERSION@|$version|g" -e "s|@TARGET@|$target_name|g" \
      -e "s|@LIBFILES@|$lib_files|g" -e "s|@SONAME@|$soname|g" \
      -e "s|@LDCONFIG@|$ldconfig|g" -e "s|@USERDIR@|$userdir|g" \
      -e "s|@STATICLIBS@|$static_libs|g" -e "s|@PROJ_VERSION@|$proj_version|g" \
      -e "s|@PROJ_VERSION_MAJOR@|$proj_major|g" -e "s|@PROJ_VERSION_MINOR@|$proj_minor|g" \
      -e "s|@PROJ_VERSION_PATCH@|$proj_patch|g" -e "s|@IMPORTED_LOCATION@|lib/$soname|g" \
      -e "s|@IMPORTED_KEY@|$imported_key|g" -e "s|@IMPORTED_VALUE@|$imported_value|g" "$1" > "$2"
}

pack() { # pack <directory name> <archive>
  local owner=(--owner=0 --group=0 --numeric-owner)
  tar --version 2>/dev/null | grep -q 'GNU tar' || owner=(--uid 0 --gid 0 --numeric-owner)
  # COPYFILE_DISABLE: no AppleDouble `._` entries from the macOS tar.
  COPYFILE_DISABLE=1 tar -C "$work" "${owner[@]}" -cf - "$1" | gzip -9n > "$2"
}

mkdir -p "$work/$cli_dir"
cp -a "$work/cli-prefix/bin" "$work/$cli_dir/bin"
licences "$work/$cli_dir"
fill "$root/tools/release/README-cli-unix.txt" "$work/$cli_dir/README.txt"
pack "$cli_dir" "$out/$cli_dir.tar.gz"

mkdir -p "$work/$lib_dir"
cp -a "$work/lib-prefix/lib" "$work/lib-prefix/include" "$work/$lib_dir/"
# pkg-config and CMake find the library as they find PROJ: lib/pkgconfig/proj.pc, lib/cmake/proj.
mkdir -p "$work/$lib_dir/lib/pkgconfig" "$work/$lib_dir/lib/cmake/proj"
fill "$root/tools/release/proj.pc.in" "$work/$lib_dir/lib/pkgconfig/proj.pc"
fill "$root/tools/release/cmake/proj-config.cmake.in" "$work/$lib_dir/lib/cmake/proj/proj-config.cmake"
fill "$root/tools/release/cmake/proj-config-version.cmake.in" "$work/$lib_dir/lib/cmake/proj/proj-config-version.cmake"
# The database is inside the library; the copy is for programs that look for the file before they
# call PROJ, as pyproj does (docs/CLIENTS-pyproj.md). It is the same proj.db, byte for byte.
mkdir -p "$work/$lib_dir/share/proj"
cp "$root/data/proj.db" "$work/$lib_dir/share/proj/proj.db"
licences "$work/$lib_dir"
fill "$root/tools/release/README-libproj-unix.txt" "$work/$lib_dir/README.txt"
pack "$lib_dir" "$out/$lib_dir.tar.gz"

ls -l "$out/$cli_dir.tar.gz" "$out/$lib_dir.tar.gz"
