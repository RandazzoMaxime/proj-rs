#!/usr/bin/env bash
# Smoke test of the two archives of one platform (Linux, macOS), as a user would use them: unpack
# each in an empty folder, with no PROJ_DATA and no grid anywhere, run the tools by their names
# and as `proj-rs <tool>`, compile and run a C program against the library. Windows:
# tools/release-smoke.ps1.
#
# Usage: tools/release-smoke.sh <cli.tar.gz> <libproj.tar.gz | ->
#   "-" in place of the C library: a platform with the tools only (linux-x86_64-musl).
#
# The expected values are the output of PROJ 9.9.0 itself (cs2cs, proj, invproj, cct, geod,
# invgeod, projinfo built from the tag by tools/build-reference.sh). Exit status: 0 if every
# check passed, 1 otherwise; a failing check does not stop the run.

set -uo pipefail

[ $# -eq 2 ] || { echo "usage: tools/release-smoke.sh <cli.tar.gz> <libproj.tar.gz>" >&2; exit 2; }
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/cli" "$work/lib" "$work/home"
tar -xzf "$1" -C "$work/cli" || exit 1
if [ "$2" != - ]; then tar -xzf "$2" -C "$work/lib" || exit 1; fi
cli=$(echo "$work/cli"/*)
lib=$(echo "$work/lib"/*)

# No data directory, no user grid: the catalogue must come from the executable.
unset PROJ_DATA PROJ_LIB PROJ_NETWORK PROJ_USER_WRITABLE_DIRECTORY
export HOME=$work/home XDG_DATA_HOME=$work/home/xdg PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES

failures=0
check() { # check <label> <expected> <actual>
  if [ "$2" = "$3" ]; then
    echo "ok   $1"
  else
    echo "FAIL $1"
    printf '  expected: %s\n  actual:   %s\n' "$2" "$3"
    failures=$((failures + 1))
  fi
}
require() { # require <label> <command...>: the command must succeed
  local label=$1
  shift
  if "$@" > /dev/null 2>&1; then echo "ok   $label"; else echo "FAIL $label"; failures=$((failures + 1)); fi
}
tab=$(printf '\t')

echo "== command-line tools: $(basename "$cli")"
for file in README.txt LICENSE NOTICE.md data/COPYING data/licenses/Apache-2.0-Esri.txt data/licenses/CC-BY-4.0-NKG.txt bin/proj-rs; do
  require "file $file" test -s "$cli/$file"
done
check "README.txt has no placeholder left" "0" "$(grep -c '@[A-Z]*@' "$cli/README.txt")"
tools=$("$cli/bin/proj-rs" --list | tr '\n' ' ')
check "proj-rs --list" "cct cs2cs geod gie invgeod invproj proj projinfo " "$tools"
for tool in $tools; do
  check "bin/$tool is a link to proj-rs" "proj-rs" "$(readlink "$cli/bin/$tool" || echo none)"
done
check "proj-rs --version" "proj-rs: Rel. 9.9.0, September 15th, 2026" "$("$cli/bin/proj-rs" --version 2>&1)"
# cs2cs prints its version on stderr, as PROJ does.
check "cs2cs --version" "Rel. 9.9.0, September 15th, 2026" "$("$cli/bin/cs2cs" --version 2>&1 | head -1)"
check "projinfo --version" "projinfo: Rel. 9.9.0, September 15th, 2026" "$("$cli/bin/projinfo" --version 2>&1)"
check "cct --version" "cct: Rel. 9.9.0, September 15th, 2026" "$("$cli/bin/cct" --version 2>&1)"

# The same cases, by the name of each tool and by `proj-rs <tool>`; `bin` is copied to
# another place first, to check that the relative links survive a move.
cp -a "$cli/bin" "$work/moved-bin"
run_cases() { # run_cases <label> <command prefix for a tool name: "" or "proj-rs ">
  local label=$1 via=$2 bin=$work/moved-bin
  tool() { local name=$1; shift; if [ -n "$via" ]; then "$bin/proj-rs" "$name" "$@"; else "$bin/$name" "$@"; fi; }
  check "$label cs2cs EPSG:4326 -> EPSG:2154" "648237.302${tab}6862271.682 0.000" \
    "$(echo "48.8584 2.2945" | tool cs2cs -d 3 EPSG:4326 EPSG:2154)"
  check "$label cs2cs longlat -> EPSG:2154" "648237.302${tab}6862271.682 0.000" \
    "$(echo "2.2945 48.8584" | tool cs2cs -d 3 +proj=longlat +datum=WGS84 +to EPSG:2154)"
  check "$label proj utm 31" "426857.99${tab}5427937.52" \
    "$(echo "2 49" | tool proj +proj=utm +zone=31 +ellps=WGS84)"
  check "$label invproj utm 31" "3.000000${tab}45.153477" \
    "$(echo "500000 5000000" | tool invproj +proj=utm +zone=31 +ellps=WGS84 -f %.6f)"
  check "$label cct utm 31" "  426857.9877   5427937.5233        0.0000        0.0000" \
    "$(echo "2 49 0 0" | tool cct +proj=utm +zone=31 +ellps=GRS80)"
  check "$label geod forward" "49.632${tab}2.979${tab}-134.258" \
    "$(echo "49 2 45 100000" | tool geod +ellps=WGS84 -f %.3f)"
  check "$label invgeod" "17.482${tab}-161.752${tab}233720.037" \
    "$(echo "49 2 51 3" | tool invgeod +ellps=WGS84 -f %.3f)"
  check "$label projinfo EPSG:32631" "+proj=utm +zone=31 +datum=WGS84 +units=m +no_defs +type=crs" \
    "$(tool projinfo -q -o PROJ EPSG:32631)"
  check "$label projinfo EPSG:2154" "+proj=lcc +lat_0=46.5 +lon_0=3 +lat_1=49 +lat_2=44 +x_0=700000 +y_0=6600000 +ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs +type=crs" \
    "$(tool projinfo -q -o PROJ EPSG:2154)"
}
run_cases "by name:" ""
run_cases "proj-rs <tool>:" "proj-rs "
check "PATH lookup of cs2cs" "$work/moved-bin/cs2cs" "$(PATH=$work/moved-bin:$PATH command -v cs2cs)"
# No TLS library: the network feature is not compiled in.
if [ "$(uname -s)" = Darwin ]; then
  linked=$(otool -L "$cli/bin/proj-rs")
else
  linked=$(ldd "$cli/bin/proj-rs")
fi
check "proj-rs links no TLS library" "0" "$(echo "$linked" | grep -ciE 'ssl|crypto|Security.framework')"

# The oldest glibc the build runs on: ubuntu-22.04 builds, glibc 2.35.
glibc_max() { objdump -T "$1" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -V | tail -1; }
# A static executable (musl) has no program interpreter and needs no C library at all.
static=no
if [ "$(uname -s)" = Linux ] && ! readelf -l "$cli/bin/proj-rs" | grep -q 'INTERP'; then
  static=yes
  check "proj-rs is a static executable" "0" "$(readelf -d "$cli/bin/proj-rs" 2>/dev/null | grep -c NEEDED)"
fi
if [ "$(uname -s)" = Linux ] && [ "$static" = no ]; then
  check "proj-rs needs glibc <= 2.35" "ok" "$([ "$(printf '%s\n2.35\n' "$(glibc_max "$cli/bin/proj-rs")" | sort -V | tail -1)" = 2.35 ] && echo ok || glibc_max "$cli/bin/proj-rs")"
fi

if [ "$2" = - ]; then
  if [ "$failures" -ne 0 ]; then
    echo "$failures check(s) failed" >&2
    exit 1
  fi
  echo "all checks passed (tools only: no C library for this platform)"
  exit 0
fi

echo "== C library: $(basename "$lib")"
case "$(uname -s)" in
  Darwin) real=libproj.25.dylib; link=libproj.dylib ;;
  *) real=libproj.so.25; link=libproj.so ;;
esac
for file in README.txt LICENSE NOTICE.md data/COPYING lib/$real lib/libproj.a lib/pkgconfig/proj.pc lib/cmake/proj/proj-config.cmake lib/cmake/proj/proj-config-version.cmake include/proj.h include/proj_constants.h include/proj_experimental.h include/proj_symbol_rename.h; do
  require "file $file" test -s "$lib/$file"
done
check "README.txt has no placeholder left" "0" "$(grep -c '@[A-Z]*@' "$lib/README.txt")"
check "share/proj/proj.db is the database of the repository" "$(cksum < "$root/data/proj.db")" \
  "$(cksum < "$lib/share/proj/proj.db" 2>/dev/null || echo missing)"
check "proj.pc and the CMake files have no placeholder left" "0" \
  "$(cat "$lib/lib/pkgconfig/proj.pc" "$lib"/lib/cmake/proj/*.cmake | grep -c '@[A-Z_]*@')"
check "lib/$link is a link to $real" "$real" "$(readlink "$lib/lib/$link" || echo none)"
for header in proj.h proj_constants.h proj_experimental.h proj_symbol_rename.h geodesic.h; do
  check "include/$header is the header of the repository" "$(cksum < "$root/capi/include/$header")" "$(cksum < "$lib/include/$header")"
done
if [ "$(uname -s)" = Darwin ]; then
  check "install name" "@rpath/libproj.25.dylib" "$(otool -D "$lib/lib/$real" | tail -1)"
  exported=$(nm -gU "$lib/lib/$real" | grep -cE ' _(proj_|pj_release)')
else
  check "soname" "libproj.so.25" "$(readelf -d "$lib/lib/$real" | sed -n 's/.*(SONAME).*\[\(.*\)\]/\1/p')"
  exported=$(nm -D --defined-only "$lib/lib/$real" | grep -cE ' (proj_|pj_release)')
  check "libproj.so.25 needs glibc <= 2.35" "ok" "$([ "$(printf '%s\n2.35\n' "$(glibc_max "$lib/lib/$real")" | sort -V | tail -1)" = 2.35 ] && echo ok || glibc_max "$lib/lib/$real")"
fi
check "exported functions of proj.h (293 = 292 and pj_release)" "ok" "$([ "$exported" -ge 293 ] && echo ok || echo "$exported")"

expected_c="version 9.9.0
lambert93 648237.302 6862271.682
inverse 48.858400 2.294500
utm31 426857.988 5427937.523"
mkdir "$work/c"
cc -I "$lib/include" "$root/tools/release/smoke.c" -o "$work/c/smoke-dynamic" -L "$lib/lib" -lproj -Wl,-rpath,"$lib/lib" 2> "$work/c/dynamic.log" \
  || { cat "$work/c/dynamic.log"; failures=$((failures + 1)); }
check "C program linked to libproj" "$expected_c" "$("$work/c/smoke-dynamic" 2>&1)"
# The static library needs the system libraries Rust's std uses; the README of the archive gives them.
static_libs=$(sed -n 's|^ *cc demo.c -Iinclude lib/libproj.a ||p' "$lib/README.txt")
check "README names the libraries of libproj.a" "ok" "$([ -n "$static_libs" ] && echo ok || echo empty)"
# shellcheck disable=SC2086
cc -I "$lib/include" "$root/tools/release/smoke.c" "$lib/lib/libproj.a" $static_libs -o "$work/c/smoke-static" 2> "$work/c/static.log" \
  || { cat "$work/c/static.log"; failures=$((failures + 1)); }
check "C program linked to libproj.a" "$expected_c" "$("$work/c/smoke-static" 2>&1)"

# pkg-config and CMake find the library as they find PROJ 9.9.0, from a copy of the archive moved
# elsewhere: the files must follow the folder, not the place the archive was packed.
cp -a "$lib" "$work/moved-lib"
moved=$work/moved-lib
if command -v pkg-config > /dev/null; then
  export PKG_CONFIG_PATH=$moved/lib/pkgconfig
  check "pkg-config --modversion proj" "9.9.0" "$(pkg-config --modversion proj)"
  check "pkg-config --atleast-version 9.9 proj" "ok" "$(pkg-config --atleast-version 9.9 proj && echo ok)"
  # shellcheck disable=SC2046
  cc "$root/tools/release/smoke.c" $(pkg-config --cflags --libs proj) -Wl,-rpath,"$moved/lib" \
    -o "$work/c/smoke-pkgconfig" 2> "$work/c/pkgconfig.log" || { cat "$work/c/pkgconfig.log"; failures=$((failures + 1)); }
  check "C program built with pkg-config" "$expected_c" "$("$work/c/smoke-pkgconfig" 2>&1)"
  check "pkg-config --static names the libraries of libproj.a" "ok" \
    "$(pkg-config --static --libs proj | grep -q -- "$(echo "$static_libs" | awk '{print $NF}')" && echo ok)"
  unset PKG_CONFIG_PATH
else
  echo "FAIL pkg-config is not installed: the proj.pc check cannot run"
  failures=$((failures + 1))
fi
if command -v cmake > /dev/null; then
  cmake -S "$root/tools/release/cmake-smoke" -B "$work/cmake" -DCMAKE_PREFIX_PATH="$moved" \
    -DCMAKE_BUILD_TYPE=Release > "$work/cmake.log" 2>&1 \
    && cmake --build "$work/cmake" >> "$work/cmake.log" 2>&1 \
    || { cat "$work/cmake.log"; failures=$((failures + 1)); }
  check "find_package(PROJ 9.9 CONFIG) reports the version" "-- PROJ_VERSION=9.9.0" \
    "$(grep -- '-- PROJ_VERSION=' "$work/cmake.log" | head -1)"
  check "C program built with CMake, PROJ::proj" "$expected_c" "$("$work/cmake/smoke" 2>&1)"
  # A request for another major version is refused, as PROJ's package refuses it.
  mkdir -p "$work/cmake-v8"
  printf 'cmake_minimum_required(VERSION 3.16)\nproject(v8 C)\nfind_package(PROJ 8 CONFIG QUIET)\nmessage(STATUS "found=${PROJ_FOUND}")\n' \
    > "$work/cmake-v8/CMakeLists.txt"
  check "find_package(PROJ 8) is refused" "-- found=0" \
    "$(cmake -S "$work/cmake-v8" -B "$work/cmake-v8/b" -DCMAKE_PREFIX_PATH="$moved" 2>&1 | grep -- '-- found=' | head -1)"
else
  echo "FAIL cmake is not installed: the CMake package check cannot run"
  failures=$((failures + 1))
fi

if [ "$failures" -ne 0 ]; then
  echo "$failures check(s) failed" >&2
  exit 1
fi
echo "all checks passed"
