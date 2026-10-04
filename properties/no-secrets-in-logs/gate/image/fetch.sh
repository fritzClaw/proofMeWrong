#!/usr/bin/env bash
# Download the pinned tool archives into vendor/ and check their hashes.
# Verus rolling releases are deleted upstream after a while, so keep vendor/
# (or the built image) archived together with the verdicts.
set -euo pipefail
cd "$(dirname "$0")"
source versions.env
mkdir -p vendor
fetch() { # url file sha256
  [ -f "vendor/$2" ] || curl -fsSL -o "vendor/$2" "$1"
  echo "$3  vendor/$2" | sha256sum -c -
}
fetch "https://github.com/verus-lang/verus/releases/download/release/rolling/$VERUS_VERSION/verus-$VERUS_VERSION-x86-linux.zip" \
  verus.zip "$VERUS_SHA256"
fetch "https://github.com/github/codeql-action/releases/download/$CODEQL_BUNDLE/codeql-bundle-linux64.tar.zst" \
  codeql.tar.zst "$CODEQL_SHA256"
