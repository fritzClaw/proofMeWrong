#!/usr/bin/env bash
# Create a pipeline run directory from the template.
# Usage: new-project.sh <run-dir> <requirements.md>
set -euo pipefail
PKG="$(cd "$(dirname "$0")/.." && pwd)"
RUN="$1"; REQ="$2"
[ -e "$RUN" ] && { echo "refusing to overwrite $RUN" >&2; exit 1; }
mkdir -p "$RUN"
cp -r "$PKG/template/." "$RUN/"
cp "$REQ" "$RUN/requirements.md"
mkdir -p "$RUN/app/src"
echo "created $RUN — next: step 1 writes $RUN/classification.toml, then run tools/freeze.sh $RUN"
