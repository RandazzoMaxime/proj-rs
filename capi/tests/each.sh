#!/usr/bin/env bash
# Run each gtest case of a binary in its own process, and print the cases that fail.
#
# Usage: capi/tests/each.sh BINARY [gtest filter]
# A crash (segmentation fault, abort) in one case does not hide the others. Run the
# environment of run.sh first (PROJ_DATA, LD_LIBRARY_PATH...): this script only loops.
set -u
binary=$1
filter=${2:-*}
passed=0
failed=()
suite=
cases=()
while IFS= read -r line; do
  case $line in
    "  "*) cases+=("${suite}${line#"${line%%[![:space:]]*}"}") ;;
    *.) suite=${line} ;;
  esac
done < <("$binary" --gtest_list_tests --gtest_filter="$filter" | sed 's/  *#.*//')
for name in "${cases[@]}"; do
  if timeout 300 "$binary" --gtest_filter="$name" >/dev/null 2>&1; then
    passed=$((passed + 1))
  else
    failed+=("$name")
  fi
done
echo "passed: $passed, failed: ${#failed[@]}"
printf '  FAILED %s\n' "${failed[@]}"
[ ${#failed[@]} -eq 0 ]
