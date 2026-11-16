#!/usr/bin/env bash
# Compare our cs2cs with the PROJ 9.9.0 cs2cs, case by case.
#
# Usage: tools/compare-cs2cs.sh [-c CASES] [-t TOL] [-r REF_DIR] [-k FILTER]
#                               [-o KEEP_DIR] [-1] [--no-build]
#
# A case is one line of CASES: SRC|DST|cs2cs options|points separated by ';'
# (each point: x y [z [t]], in the axis order of the CRS). Lines starting with
# '#' are skipped. SRC and DST are split on blanks, as a shell would pass them;
# -1 passes each as one argument instead (PROJ joins the '+' words of several
# arguments, our port wraps a white-space argument in quotes). Both binaries
# get the same arguments, the same stdin, and `-d 10` first. PROJ_DATA is the
# reference database, then this repository's data/, then data/tests (the
# reference is built without TIFF: it cannot read the .tif grids, our port can).
#
# Status, from the best to the worst:
#   same    stdout identical, byte for byte
#   tol     numeric difference, all <= TOL (in output units)
#   text    other text difference (format, field count, error text)
#   num     numeric difference > TOL
#   lines   number of output lines differs
#   status  an error on one side only, or exit codes differ
#
# Output: a TSV (pair, status, max |delta| per field, reference line, our
# line, exit codes), then a count per status after a '#' line. -o keeps the
# raw outputs per case.
set -uo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cases="$root/tools/compare-cs2cs/cases.txt"
awk_prog="$root/tools/compare-cs2cs/compare.awk"
tol=1e-6
ref=""
filter=""
keep=""
build=1
one_argv=0

while [ $# -gt 0 ]; do
    case $1 in
        -c) cases=$2; shift 2 ;;
        -t) tol=$2; shift 2 ;;
        -r) ref=$2; shift 2 ;;
        -k) filter=$2; shift 2 ;;
        -o) keep=$2; shift 2 ;;
        -1) one_argv=1; shift ;;
        --no-build) build=0; shift ;;
        -h|--help) sed -n '2,/^set -uo/p' "$0" | sed '$d'; exit 0 ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done

if [ -z "$ref" ]; then
    common=$(git -C "$root" rev-parse --path-format=absolute --git-common-dir 2>/dev/null || true)
    for candidate in "$root/target/proj-reference" "$(dirname "${common:-/nonexistent}")/target/proj-reference"; do
        if [ -x "$candidate/build/bin/cs2cs" ]; then
            ref=$candidate
            break
        fi
    done
fi
if [ -z "$ref" ] || [ ! -x "$ref/build/bin/cs2cs" ]; then
    echo "reference cs2cs not found: run tools/build-reference.sh or pass -r DIR" >&2
    exit 2
fi
ref=$(cd "$ref" && pwd)

if [ "$build" = 1 ]; then
    (cd "$root" && cargo build --release --bin cs2cs) >&2 || exit 2
fi
ours="${CARGO_TARGET_DIR:-$root/target}/release/cs2cs"
if [ ! -x "$ours" ]; then
    echo "our cs2cs not found: $ours" >&2
    exit 2
fi

pdata="$ref/build/data:$root/data:$root/data/tests"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
[ -n "$keep" ] && mkdir -p "$keep"

{
printf 'pair\tstatus\tmax_delta\tref_line\tport_line\trc_ref/rc_port\n'
index=0
while IFS='|' read -r src dst opts pts || [ -n "$src" ]; do
    case $src in ''|'#'*) continue ;; esac
    if [ -n "$filter" ] && [[ "$src|$dst|$opts|$pts" != *"$filter"* ]]; then
        continue
    fi
    index=$((index + 1))
    read -ra optarr <<< "$opts"
    if [ "$one_argv" = 1 ]; then srcarr=("$src"); dstarr=("$dst"); else read -ra srcarr <<< "$src"; read -ra dstarr <<< "$dst"; fi
    printf '%s\n' "$pts" | tr ';' '\n' | sed 's/^ *//;s/ *$//' | grep -v '^$' > "$tmp/in"

    LD_LIBRARY_PATH="$ref/build/lib" PROJ_DATA="$pdata" PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
        "$ref/build/bin/cs2cs" -d 10 ${optarr[@]+"${optarr[@]}"} "${srcarr[@]}" +to "${dstarr[@]}" \
        < "$tmp/in" > "$tmp/ref.out" 2> "$tmp/ref.err"
    rcr=$?
    PROJ_DATA="$pdata" PROJ_SKIP_READ_USER_WRITABLE_DIRECTORY=YES \
        "$ours" -d 10 ${optarr[@]+"${optarr[@]}"} "${srcarr[@]}" +to "${dstarr[@]}" \
        < "$tmp/in" > "$tmp/port.out" 2> "$tmp/port.err"
    rcp=$?

    pair="$src -> $dst"
    [ -n "$opts" ] && pair="$pair [$opts]"
    awk -v refFile="$tmp/ref.out" -v portFile="$tmp/port.out" -v tol="$tol" \
        -v rcr="$rcr" -v rcp="$rcp" -v pair="$pair" -f "$awk_prog"

    if [ -n "$keep" ]; then
        name=$(printf 'case-%03d' "$index")
        cp "$tmp/ref.out" "$keep/$name.ref.out"
        cp "$tmp/ref.err" "$keep/$name.ref.err"
        cp "$tmp/port.out" "$keep/$name.port.out"
        cp "$tmp/port.err" "$keep/$name.port.err"
        printf '%s\t%s -> %s\t%s\n' "$name" "$src" "$dst" "$opts" >> "$keep/index.tsv"
    fi
done
} < "$cases" > "$tmp/summary.tsv"
cat "$tmp/summary.tsv"

echo "#"
echo "# cases: $(($(wc -l < "$tmp/summary.tsv") - 1))"
for status in same tol text num lines status; do
    count=$(awk -F'\t' -v s="$status" 'NR > 1 && $2 == s' "$tmp/summary.tsv" | wc -l)
    echo "# $status: $count"
done
