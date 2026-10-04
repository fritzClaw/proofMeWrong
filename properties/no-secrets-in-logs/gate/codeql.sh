#!/usr/bin/env bash
# CodeQL part of the gate. Builds a database of the run, adds the package
# models plus project models generated from the frozen classification, and
# runs rust/cleartext-logging and the custom choke-point query.
# Usage: codeql.sh <run-dir> <out-dir>     (writes <out-dir>/codeql.csv)
set -euo pipefail
PKG="$(cd "$(dirname "$0")/.." && pwd)"
RUN="$(cd "$1" && pwd)"
OUT="$(mkdir -p "$2" && cd "$2" && pwd)"
CODEQL="${CODEQL:-codeql}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cp -r "$PKG/codeql/models" "$WORK/models"
cp -r "$PKG/codeql/queries" "$WORK/queries"
mkdir -p "$WORK/models/generated"
python3 "$PKG/codegen/gen_codeql_models.py" "$RUN/classification.toml" "$WORK/models/generated/schema.model.yml"
"$CODEQL" database create "$WORK/db" --language=rust --source-root="$RUN" --overwrite >"$OUT/codeql-create.log" 2>&1
# Without a loaded cargo workspace the extractor skips macro expansion and
# sees nothing inside verus! -- the analysis would then be silently empty.
if grep -q "failed to load workspace\|semantic analyzer unavailable" "$OUT/codeql-create.log"; then
  echo "codeql: extractor could not load the cargo workspace (analysis would be empty):" >&2
  grep "failed to load workspace\|semantic analyzer unavailable" "$OUT/codeql-create.log" | head -5 >&2
  exit 1
fi
"$CODEQL" database analyze "$WORK/db" \
  codeql/rust-queries:queries/security/CWE-312/CleartextLogging.ql \
  "$WORK/queries/OutputOutsideChokePoints.ql" \
  --additional-packs="$WORK" \
  --model-packs=proofmewrong/no-secrets-in-logs-models \
  --format=csv --output="$OUT/codeql.csv" >"$OUT/codeql-analyze.log" 2>&1
echo "codeql: $(wc -l < "$OUT/codeql.csv") alert(s)"
