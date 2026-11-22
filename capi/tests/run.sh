#!/usr/bin/env bash
# Compile the C tests of PROJ 9.9.0 against the C library of proj-rs, and run them.
#
# Usage: capi/tests/run.sh [--release] [TEST...]
#   TEST is one of: symbols abi lists intermediate_crs errno_string angular_io context include_h
#   fork datadir geodesic proj_string_type c_api
#   (default: all). PENDING=0 runs the cases of pending.txt too, to see them fail.
#   GTEST_FILTER=pattern narrows the gtest cases of the C++ tests.
#
# Needs: g++ (C++17), gcc, the SQLite3 headers, and googletest's sources (Debian: libgtest-dev,
# which installs them under /usr/src/googletest; or GTEST_SRC=<googletest/googletest dir>).
# PROJ_REFERENCE is a PROJ 9.9.0 tree built by tools/build-reference.sh (default:
# target/proj-reference): the sources of the tests and the data directory come from it. The
# tests are not changed, except that `test_c_api.cpp` loses the cases listed in
# capi/tests/excluded.txt (they call the C++ API of PROJ), see capi/tests/README.md.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
here=$root/capi/tests
profile_arg=()
if [ "${1:-}" = "--release" ]; then
  profile_arg=(--release)
  shift
fi
tests=("$@")
[ ${#tests[@]} -gt 0 ] || tests=(symbols abi lists intermediate_crs errno_string angular_io
                                  context include_h fork datadir geodesic proj_string_type c_api)

ref=${PROJ_REFERENCE:-$root/target/proj-reference}
src=$ref/src
unit=$src/test/unit
out=${CAPI_TEST_OUT:-$root/target/capi-tests}
dist=$out/dist
mkdir -p "$out"

for needed in "$unit/test_c_api.cpp" "$ref/build/data/for_tests/proj.db"; do
  [ -e "$needed" ] || { echo "missing $needed: run tools/build-reference.sh" >&2; exit 2; }
done
cmp -s "$src/src/proj.h" "$root/capi/include/proj.h" || {
  echo "capi/include/proj.h differs from the proj.h of the reference" >&2; exit 2; }

"$here/dist.sh" ${profile_arg[@]+"${profile_arg[@]}"} "$dist" >/dev/null

gtest=${GTEST_SRC:-}
for candidate in /usr/src/googletest/googletest /usr/src/gtest; do
  [ -n "$gtest" ] || { [ -d "$candidate/src" ] && gtest=$candidate; }
done
[ -n "$gtest" ] || { echo "googletest sources not found (set GTEST_SRC)" >&2; exit 2; }
if [ ! -f "$out/gtest-all.o" ]; then
  g++ -std=c++17 -O1 -I"$gtest/include" -I"$gtest" -c "$gtest/src/gtest-all.cc" -o "$out/gtest-all.o"
fi

# cs2cs and projinfo of the reference, found through PATH from outside the repository.
bin=$out/refbin
mkdir -p "$bin"
for tool in "$ref"/build/bin/*; do ln -sf "$tool" "$bin/$(basename "$tool")"; done

cxxflags=(-std=c++17 -O1 -I"$root/capi/include" -I"$src/include" -I"$ref/build/src"
          -I"$src/src" -I"$unit" -I"$gtest/include")
ldflags=(-L"$dist/lib" -lproj -lsqlite3 -lpthread -Wl,-rpath,"$dist/lib")
export PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES
export PROJ_DATA=$ref/build/data/for_tests
export PROJ_SOURCE_DATA=$src/data
export PATH=$bin:$PATH
export LD_LIBRARY_PATH=$dist/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
# excluded.txt: cases that cannot pass (they call the C++ API or read the internals of PROJ).
# pending.txt: cases that do not pass yet, each with its reason. Both are skipped at run time,
# except that the `CApi.` cases of excluded.txt are cut out of the source by filter_tests.py
# (they do not link). PENDING=0 keeps the pending cases in the run.
skipped=$(sed 's/#.*//' "$here/excluded.txt" | grep -v '^CApi\.' | tr -s ' \n' ':' || true)
if [ "${PENDING:-1}" != 0 ]; then
  skipped=$skipped$(sed 's/#.*//' "$here/pending.txt" | tr -s ' \n' ':' || true)
fi
filter=${GTEST_FILTER:-*}${skipped:+-${skipped%:}}

run_gtest() {
  local name=$1 source=$2
  g++ "${cxxflags[@]}" "$source" "$unit/main.cpp" "$out/gtest-all.o" -o "$out/$name" "${ldflags[@]}"
  echo "== $name"
  "$out/$name" --gtest_filter="$filter"
}

for test in "${tests[@]}"; do
  case $test in
    symbols)
      echo "== symbols"
      python3 "$here/check_symbols.py" "$root/capi/include/proj.h" "$dist/lib/libproj.so.25" 2>/dev/null \
        || python3 "$here/check_symbols.py" "$root/capi/include/proj.h" "$dist/lib/libproj.25.dylib" ;;
    abi)
      echo "== abi"
      for headers in "$root/capi/include" "$src/src"; do
        gcc -std=c11 -I"$headers" "$here/abi_layout.c" -o "$out/abi_layout"
        echo "layout of the header in $headers: ok"
      done ;;
    lists)
      echo "== lists"
      gcc -std=c99 -I"$root/capi/include" "$here/lists.c" -o "$out/lists_ours" "${ldflags[@]}"
      gcc -std=c99 -I"$src/src" "$here/lists.c" -o "$out/lists_ref" -L"$ref/build/lib" -lproj \
        -Wl,-rpath,"$ref/build/lib"
      "$out/lists_ours" > "$out/lists_ours.txt"
      "$out/lists_ref" > "$out/lists_ref.txt"
      diff "$out/lists_ref.txt" "$out/lists_ours.txt" | head -5
      cmp -s "$out/lists_ref.txt" "$out/lists_ours.txt" || { echo "lists differ from PROJ" >&2; exit 1; }
      echo "$(wc -l < "$out/lists_ours.txt") list entries identical to PROJ" ;;
    intermediate_crs)
      echo "== intermediate_crs"
      gcc -std=c99 -I"$root/capi/include" "$here/intermediate_crs.c" -o "$out/intermediate_ours" "${ldflags[@]}"
      gcc -std=c99 -I"$src/src" "$here/intermediate_crs.c" -o "$out/intermediate_ref" -L"$ref/build/lib" -lproj \
        -Wl,-rpath,"$ref/build/lib"
      # LD_LIBRARY_PATH names our library for the C suite; it wins over the RUNPATH of the binary.
      LD_LIBRARY_PATH="$ref/build/lib" PROJ_DATA="$ref/build/data" "$out/intermediate_ref" \
        > "$out/intermediate_ref.txt"
      "$out/intermediate_ours" > "$out/intermediate_ours.txt"
      diff "$out/intermediate_ref.txt" "$out/intermediate_ours.txt" | head -8
      cmp -s "$out/intermediate_ref.txt" "$out/intermediate_ours.txt" \
        || { echo "operations through allowed intermediate CRS differ from PROJ" >&2; exit 1; }
      cat "$out/intermediate_ours.txt" ;;
    geodesic)
      # geodesic.h, the header of PROJ 9.9.0: the layout of its structures, and geodtest.c, the C
      # tests of geodesic.c that PROJ runs, against the geod_* functions of proj-rs.
      echo "== geodesic"
      cmp -s "$root/capi/include/geodesic.h" "$src/src/geodesic.h" \
        || { echo "capi/include/geodesic.h is not the geodesic.h of the reference" >&2; exit 1; }
      gcc -std=c99 -I"$root/capi/include" -c "$here/geodesic_layout.c" -o "$out/geodesic_layout.o"
      gcc -std=c99 -I"$root/capi/include" "$src/src/tests/geodtest.c" -o "$out/geodtest" "${ldflags[@]}" -lm
      "$out/geodtest" && echo "geodtest.c of PROJ: no failure" ;;
    proj_string_type)
      echo "== proj_string_type"
      gcc -std=c99 -I"$root/capi/include" "$here/proj_string_type.c" -o "$out/type_ours" "${ldflags[@]}"
      gcc -std=c99 -I"$src/src" "$here/proj_string_type.c" -o "$out/type_ref" -L"$ref/build/lib" -lproj \
        -Wl,-rpath,"$ref/build/lib"
      # LD_LIBRARY_PATH names our library for the C suite; it wins over the RUNPATH of the binary.
      LD_LIBRARY_PATH="$ref/build/lib" PROJ_DATA="$ref/build/data" "$out/type_ref" > "$out/type_ref.txt"
      "$out/type_ours" > "$out/type_ours.txt"
      diff "$out/type_ref.txt" "$out/type_ours.txt" | head -12
      cmp -s "$out/type_ref.txt" "$out/type_ours.txt" \
        || { echo "the type of a PROJ string differs from PROJ" >&2; exit 1; }
      echo "$(wc -l < "$out/type_ours.txt") definitions read as PROJ reads them" ;;
    errno_string) run_gtest proj_errno_string_test "$unit/proj_errno_string_test.cpp" ;;
    angular_io)   run_gtest proj_angular_io_test "$unit/proj_angular_io_test.cpp" ;;
    context)      run_gtest proj_context_test "$unit/proj_context_test.cpp" ;;
    include_h)
      gcc -std=c99 -Wall -I"$root/capi/include" "$unit/include_proj_h_from_c.c" -o "$out/include_h" "${ldflags[@]}"
      echo "== include_proj_h_from_c"; "$out/include_h" && echo "ok" ;;
    fork)
      pthreads=-DPROJ_HAS_PTHREADS
      [ "$(uname -s)" != Darwin ] || pthreads=   # CMake defines it off Apple only
      gcc -std=c99 $pthreads -I"$root/capi/include" "$unit/test_fork.c" -o "$out/test_fork" "${ldflags[@]}"
      echo "== test_fork"; "$out/test_fork" && echo "ok" ;;
    datadir)
      echo "== datadir"
      gcc -std=c99 -I"$root/capi/include" "$here/datadir.c" -o "$out/datadir_ours" "${ldflags[@]}"
      gcc -std=c99 -I"$src/src" "$here/datadir.c" -o "$out/datadir_ref" -L"$ref/build/lib" -lproj \
        -Wl,-rpath,"$ref/build/lib"
      scratch=$(mktemp -d)
      mkdir -p "$scratch/home" "$scratch/prefix/lib" "$scratch/prefix/share/proj" "$scratch/elsewhere"
      # One run of the program, given a path to set, under an empty environment plus the variables.
      # The reference is run without LD_LIBRARY_PATH, which would give it our library.
      probe() { local program=$1 path=$2; shift 2; (cd "$scratch/elsewhere" && env -i "$@" "$program" "$path"); }
      compare_env() {
        local with_list=$1; shift
        local theirs ours
        theirs=$(probe "$out/datadir_ref" /tmp/added "$@") || exit 1
        ours=$(probe "$out/datadir_ours" /tmp/added LD_LIBRARY_PATH="$dist/lib" "$@") || exit 1
        [ "$(echo "$theirs" | sed -n 1p)" = "$(echo "$ours" | sed -n 1p)" ] \
          || { echo "user directory differs for $*: $theirs / $ours" >&2; exit 1; }
        [ "$(echo "$theirs" | sed -n 3p)" = "$(echo "$ours" | sed -n 3p)" ] \
          || { echo "search path with paths differs for $*: $theirs / $ours" >&2; exit 1; }
        if [ "$with_list" = list ]; then
          [ "$(echo "$theirs" | sed -n 2p)" = "$(echo "$ours" | sed -n 2p)" ] \
            || { echo "search path differs for $*: $theirs / $ours" >&2; exit 1; }
        fi
      }
      h=$scratch/home
      compare_env first HOME="$h"
      compare_env list HOME="$h" PROJ_DATA=/d1
      compare_env list HOME="$h" PROJ_DATA=/d1:/d2
      compare_env list HOME="$h" PROJ_LIB=/old
      compare_env list HOME="$h" PROJ_DATA=/d1 PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES
      compare_env first HOME="$h" XDG_DATA_HOME=/xdg
      compare_env first HOME="$h" XDG_DATA_HOME=
      compare_env first HOME="$h" PROJ_USER_WRITABLE_DIRECTORY=/chosen
      compare_env first HOME=/no/such/home
      compare_env first
      echo "user directory and search path identical to libproj in 10 environments"
      # The library finds ../share/proj of its own file, not of the executable.
      cp "$dist"/lib/* "$scratch/prefix/lib/"
      gcc -std=c99 -I"$root/capi/include" "$here/datadir.c" -o "$scratch/elsewhere/datadir_installed" \
        -L"$scratch/prefix/lib" -lproj -Wl,-rpath,"$scratch/prefix/lib"
      listed=$(env -i HOME="$h" "$scratch/elsewhere/datadir_installed" | sed -n 2p)
      case $listed in
        *:"$scratch/prefix/share/proj") echo "share/proj of the library found: $listed" ;;
        *) echo "share/proj of the library not listed: $listed" >&2; exit 1 ;;
      esac
      rm -rf "$scratch" ;;
    c_api)
      python3 "$here/filter_tests.py" "$unit/test_c_api.cpp" "$here/excluded.txt" "$out/test_c_api_filtered.cpp"
      g++ "${cxxflags[@]}" -DTIFF_ENABLED -include "$unit/gtest_include.h" "$out/test_c_api_filtered.cpp" "$unit/main.cpp" \
        "$out/gtest-all.o" -o "$out/test_c_api" "${ldflags[@]}"
      echo "== test_c_api"; "$out/test_c_api" --gtest_filter="$filter" ;;
    *) echo "unknown test $test" >&2; exit 2 ;;
  esac
done
