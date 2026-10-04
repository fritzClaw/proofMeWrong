#!/usr/bin/env bash
# Build the pinned gate image from this package.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PKG="$(cd "$HERE/../.." && pwd)"
"$HERE/fetch.sh"
CTX="$(mktemp -d)"
trap 'rm -rf "$CTX"' EXIT
cp "$HERE/Dockerfile" "$CTX/"
ln "$HERE/vendor/verus.zip" "$CTX/verus.zip" 2>/dev/null || cp "$HERE/vendor/verus.zip" "$CTX/verus.zip"
tar --zstd -xf "$HERE/vendor/codeql.tar.zst" -C "$CTX"
mkdir -p "$CTX/package"
for d in codegen codeql examples gate lib template tools; do cp -r "$PKG/$d" "$CTX/package/$d"; done
rm -rf "$CTX/package/gate/image" "$CTX/package/lib/target"
find "$CTX/package" -name __pycache__ -prune -exec rm -rf {} +
# Optional: build behind an HTTPS proxy with its own CA (EXTRA_CA_CERT=<pem>).
# This only affects how tools are downloaded, not what ends up in the image.
: > "$CTX/extra-ca.crt"
[ -n "${EXTRA_CA_CERT:-}" ] && cp "$EXTRA_CA_CERT" "$CTX/extra-ca.crt"
NET=()
[ -n "${HTTPS_PROXY:-}" ] && NET=(--network host --build-arg "HTTPS_PROXY=$HTTPS_PROXY" --build-arg "HTTP_PROXY=${HTTP_PROXY:-$HTTPS_PROXY}" --build-arg "https_proxy=$HTTPS_PROXY" --build-arg "http_proxy=${HTTP_PROXY:-$HTTPS_PROXY}")
docker build "${NET[@]}" -t nosecrets-gate:v0.1 "$CTX"
docker image inspect nosecrets-gate:v0.1 --format '{{.Id}}'
