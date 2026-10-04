#!/usr/bin/env bash
# Local check for coding agents: structure (incl. proof-cheating bans), Verus and clippy
# ban list on the app crate. This is a development aid only; its result has
# no authority. The authoritative verdict comes from gate/gate.sh, started by
# a human. CodeQL is intentionally not part of this check (INTENT.md §3.5).
# Usage: check.sh <run-dir>
set -uo pipefail
PKG="$(cd "$(dirname "$0")/.." && pwd)"
RUN="$(cd "$1" && pwd)"
VERUS_RLIMIT="${VERUS_RLIMIT:-30}"
cd "$RUN"
status=0
echo "== structure"
python3 "$PKG/tools/check_structure.py" app/src || status=1
echo "== verus (trusted library and app)"
cargo verus verify --locked -p app -- --rlimit "$VERUS_RLIMIT" || status=1
echo "== clippy ban list (app)"
cargo clippy --locked -p app --no-deps -- \
  -D clippy::disallowed_macros -D clippy::disallowed_methods -D clippy::disallowed_types -D unsafe_code || status=1
if [ "$status" -eq 0 ]; then echo "local check: OK (not a gate verdict)"; else echo "local check: FAILED"; fi
exit "$status"
