#!/usr/bin/env bash
# Réenregistre tests/parity/expected.jsonl avec la référence PROJ 9.9.0.
# Usage : PROJ_REFERENCE_DIR=/chemin/proj-reference tools/parity/record.sh
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
ref="${PROJ_REFERENCE_DIR:-$root/target/proj-reference}"
exec python3 "$here/record.py" --reference-dir "$ref" "$@"
