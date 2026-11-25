# Proof of which libproj a pass of a client bench loads. Sourced by grass.sh, pdal.sh, pyproj.sh,
# spatialite.sh and libgeotiff.sh, after `set -euo pipefail`; gmt.sh has its own copy of the
# same checks.
#
# Each bench builds its client once against PROJ 9.9.0 and runs it twice, putting a libproj.so.25
# first on LD_LIBRARY_PATH: the one of PROJ 9.9.0, then the C library of proj-rs. Nothing in the
# test outcomes proves that swap, so for each pass:
#   proof_static  ldd resolves libproj.so.25 of the client's objects to the file of the pass, and
#                 no object of their dependency chains carries a DT_RPATH (searched before
#                 LD_LIBRARY_PATH) that leads to another libproj.so.25;
#   proof_call    a real client call that reaches PROJ runs under LD_DEBUG=libs and the
#                 proj_count.c interposer: every libproj the dynamic linker initialises is the
#                 file of the pass, and the interposer counts a call into it;
#   proof_log     the interposer stays preloaded for the whole pass (proof_env): every process
#                 that mapped a libproj, at start or by dlopen, logged the file of the pass, and
#                 at least a floor of processes called into it.
# The two files must differ (proof_distinct).

proof_build() {
  local work=$1 include=$2
  cc -shared -fPIC -O2 -Wall -Wextra -I"$include" "$(dirname "${BASH_SOURCE[0]}")/proj_count.c" \
    -o "$work/proj_count.so" -ldl
  PROOF_PRELOAD=$work/proj_count.so
  PROOF_DIR=$work
}

proof_distinct() {
  local a b
  a=$(readlink -f "$1/libproj.so.25")
  b=$(readlink -f "$2/libproj.so.25")
  [ "$(sha256sum < "$a")" != "$(sha256sum < "$b")" ] \
    || { echo "the two libproj.so.25 are the same file: $a $b" >&2; exit 1; }
}

# The environment assignments of a pass: LD_LIBRARY_PATH (the directories given, in order), the
# interposer and its log.
proof_env() {
  local path=$1 log=$2
  echo "LD_LIBRARY_PATH=$path"
  echo "LD_PRELOAD=$PROOF_PRELOAD"
  echo "PROJ_COUNT_LOG=$log"
}

# proof_static NAME LIBDIR LD_LIBRARY_PATH OBJECT...
proof_static() {
  local name=$1 libdir=$2 path=$3 want obj ldd_out got rpath dir
  shift 3
  want=$(readlink -f "$libdir/libproj.so.25")
  [ -f "$want" ] || { echo "$name: no $libdir/libproj.so.25" >&2; exit 1; }
  for obj in "$@"; do
    ldd_out=$(env LD_LIBRARY_PATH="$path" ldd "$obj")
    got=$(awk '$1 == "libproj.so.25" { print $3 }' <<< "$ldd_out")
    [ -n "$got" ] && [ "$(readlink -f "$got")" = "$want" ] \
      || { echo "$name: ldd resolves libproj.so.25 of $obj to '$got', not $want" >&2; exit 1; }
    echo "$name: ldd $(basename "$obj"): libproj.so.25 => $got"
    for dep in "$obj" $(awk '$2 == "=>" && $3 ~ /^\// { print $3 }' <<< "$ldd_out"); do
      rpath=$(readelf -d "$dep" | sed -n 's/.*(RPATH).*\[\(.*\)\]/\1/p')
      [ -z "$rpath" ] && continue
      for dir in ${rpath//:/ }; do
        dir=${dir//\$ORIGIN/$(dirname "$dep")}
        if [ -e "$dir/libproj.so.25" ]; then
          echo "$name: $dep has a DT_RPATH to $dir, which holds a libproj.so.25" >&2; exit 1
        fi
      done
    done
  done
  echo "$name: no DT_RPATH that could load another libproj"
}

# proof_call NAME LIBDIR LD_LIBRARY_PATH COMMAND...  (run from the current directory)
proof_call() {
  local name=$1 libdir=$2 path=$3 want log=$PROOF_DIR/proof-$1.log ld=$PROOF_DIR/proof-$1.ld out
  shift 3
  want=$(readlink -f "$libdir/libproj.so.25")
  rm -f "$log" "$ld"
  out=$(env $(proof_env "$path" "$log") LD_DEBUG=libs LD_DEBUG_OUTPUT="$ld" "$@" 2>&1 | tail -1)
  echo "$name: $(basename "$1"): $out"
  local inits
  inits=$(cat "$ld".* | sed -n 's/.*calling init: \(.*libproj\.so[^ ]*\)$/\1/p' | sort -u)
  rm -f "$ld".*
  [ -n "$inits" ] || { echo "$name: LD_DEBUG=libs shows no libproj init" >&2; exit 1; }
  for got in $inits; do
    [ "$(readlink -f "$got")" = "$want" ] \
      || { echo "$name: LD_DEBUG=libs shows libproj init from $got, not $want" >&2; exit 1; }
    echo "$name: LD_DEBUG=libs: calling init: $got"
  done
  awk -F'\t' -v want="$want" -v name="$name" '
    $1 == "map" || $1 == "calls" { if ($5 != want) { print name ": process maps " $5 > "/dev/stderr"; bad++ } }
    $1 == "calls" { calls++; line = $6 }
    END {
      if (bad || calls == 0) { print name ": the probe did not call into " want > "/dev/stderr"; exit 1 }
      print name ": interposer: " want ", " line
    }' "$log"
}

# proof_log NAME LIBDIR LOG FLOOR: every process of the pass that mapped a libproj logged the file
# of the pass, and at least FLOOR of them called into it.
proof_log() {
  local name=$1 libdir=$2 log=$3 floor=$4 want
  want=$(readlink -f "$libdir/libproj.so.25")
  [ -s "$log" ] || { echo "$name: the interposer logged nothing ($log)" >&2; exit 1; }
  awk -F'\t' -v want="$want" -v name="$name" -v floor="$floor" '
    $1 == "map" || $1 == "calls" { if ($5 != want) { other[$5]++; bad++ } }
    $1 == "map" { maps++ }
    $1 == "calls" { callers++ }
    END {
      for (p in other) print name ": " other[p] " processes mapped " p > "/dev/stderr"
      if (bad) exit 1
      if (callers < floor) { print name ": only " callers+0 " processes called into libproj (floor " floor ")" > "/dev/stderr"; exit 1 }
      print name ": interposer: " maps+0 " processes mapped a libproj at start, " callers " called into it, all " want
    }' "$log"
}
