#!/usr/bin/env bash
# Lance une ou plusieurs cibles de fuzz en parallèle, chacune pour SECONDES (voir docs/FUZZ.md).
#   fuzz/run.sh SECONDES [cible...]      sans cible : les cinq
# Prérequis : cargo +nightly fuzz build, python3 fuzz/seed.py. Journaux : fuzz/logs/<cible>.log.
set -euo pipefail
cd "$(dirname "$0")"
seconds="${1:?durée en secondes}"
shift
targets=("$@")
[ ${#targets[@]} -gt 0 ] || targets=(proj_string user_input transform grid_reader model_files)
mkdir -p logs
for target in "${targets[@]}"; do
  case "$target" in
    proj_string) max_len=512 ;;
    user_input) max_len=2048 ;;
    transform) max_len=256 ;;
    grid_reader) max_len=65536 ;;
    model_files) max_len=8192 ;;
    *) echo "cible inconnue : $target" >&2; exit 2 ;;
  esac
  mkdir -p "corpus/$target" "artifacts/$target"
  "./target/x86_64-unknown-linux-gnu/release/$target" \
    -max_total_time="$seconds" -timeout=10 -rss_limit_mb=2048 -max_len="$max_len" \
    -artifact_prefix="artifacts/$target/" "corpus/$target" > "logs/$target.log" 2>&1 &
done
wait || true
for target in "${targets[@]}"; do
  echo "== $target"
  grep -E "^#[0-9]+\s+(DONE|REDUCE)|ERROR: |SUMMARY|Test unit written" "logs/$target.log" | tail -3
done
