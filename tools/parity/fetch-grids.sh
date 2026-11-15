#!/usr/bin/env bash
# Recrée le dossier des grilles du corpus avec grilles (tests/parity/grids/grids.txt).
# Usage : tools/parity/fetch-grids.sh [DOSSIER]    (défaut : target/parity-grids)
# Chaque fichier vient de https://cdn.proj.org/<nom> et doit avoir la taille listée.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
dir="${1:-$root/target/parity-grids}"
list="$root/tests/parity/grids/grids.txt"
mkdir -p "$dir"
while IFS=$'\t' read -r name bytes; do
  case "$name" in ''|'#'*) continue ;; esac
  target="$dir/$name"
  if [ -f "$target" ] && [ "$(wc -c < "$target")" -eq "$bytes" ]; then
    continue
  fi
  curl -fsSL --retry 3 "https://cdn.proj.org/$name" -o "$target.part"
  got="$(wc -c < "$target.part")"
  if [ "$got" -ne "$bytes" ]; then
    rm -f "$target.part"
    echo "taille inattendue pour $name : $got au lieu de $bytes" >&2
    exit 1
  fi
  mv "$target.part" "$target"
done < "$list"
echo "$(find "$dir" -maxdepth 1 -type f | wc -l) fichiers dans $dir"
