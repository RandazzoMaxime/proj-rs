#!/usr/bin/env bash
# Download the PROJ grids listed in scripts/grids.txt into data/grids.
#
#   scripts/fetch-grids.sh            # fetch what is missing
#   GRID_CDN=https://mirror scripts/fetch-grids.sh
#
# Files already present are kept. Exit status is 1 if any download failed, so
# a wrong name or a CDN outage is visible instead of silently skipping tests.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cdn="${GRID_CDN:-https://cdn.proj.org}"
dest="$root/data/grids"
mkdir -p "$dest"

failed=0
while IFS= read -r name; do
  case "$name" in ''|'#'*) continue ;; esac
  if [ -s "$dest/$name" ]; then
    echo "ok       $name (already there)"
    continue
  fi
  if curl --fail --silent --show-error --location --retry 3 --retry-delay 2 \
      --output "$dest/$name.part" "$cdn/$name"; then
    mv "$dest/$name.part" "$dest/$name"
    echo "fetched  $name ($(wc -c <"$dest/$name") bytes)"
  else
    rm -f "$dest/$name.part"
    echo "FAILED   $name" >&2
    failed=1
  fi
done <"$root/scripts/grids.txt"

exit "$failed"
