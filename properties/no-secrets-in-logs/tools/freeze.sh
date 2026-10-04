#!/usr/bin/env bash
# Freeze a run after step 1: validate the classification, generate the
# project-specific trusted library, and record hashes of all protected paths.
# Usage: freeze.sh <run-dir>
set -euo pipefail
PKG="$(cd "$(dirname "$0")/.." && pwd)"
RUN="$(cd "$1" && pwd)"
[ -f "$RUN/classification.toml" ] || { echo "missing $RUN/classification.toml" >&2; exit 1; }
[ -e "$RUN/freeze.json" ] && { echo "already frozen: $RUN/freeze.json" >&2; exit 1; }
rm -rf "$RUN/trusted"
mkdir -p "$RUN/trusted"
cp -r "$PKG/lib" "$RUN/trusted/nosecrets"
rm -rf "$RUN/trusted/nosecrets/target"
python3 "$PKG/codegen/gen_schema.py" "$RUN/classification.toml" "$RUN/trusted/nosecrets/src/schema.rs"
python3 "$PKG/tools/protected.py" record "$RUN"
echo "frozen: $RUN/freeze.json"
