# DevContainer for proofMeWrong: every external dependency of the
# no-secrets-in-logs pipeline, in the versions pinned in
# properties/no-secrets-in-logs/gate/image/versions.env (keep them in sync;
# `run-pipeline.sh doctor` checks them).
#
#   Verus 0.2026.10.04.fc7d32e (with bundled Z3), Rust 1.98.1 (+ clippy,
#   rustfmt, rust-src), cargo-deny 0.18.9, CodeQL bundle 2.27.1,
#   Python 3.11, Node.js 22 + Claude Code CLI (the agents of steps 1 and 2).
#
# Verus publishes only rolling releases and deletes old ones. If the pinned
# archive is gone upstream, put it into .devcontainer/vendor/verus.zip (same
# SHA-256) and rebuild; the same works for vendor/codeql.tar.zst.

# python:3.11-trixie (Verus needs glibc >= 2.39), pinned by digest, pulled via
# the Docker Hub mirror on mirror.gcr.io.
ARG BASE=mirror.gcr.io/library/python@sha256:27e044f7e01fea05c1760324d58fc5360a0767b9ef098e74ddaf8c70b8f46d26
FROM ${BASE}

ARG RUST_TOOLCHAIN=1.98.1
ARG CARGO_DENY_VERSION=0.18.9
ARG VERUS_VERSION=0.2026.10.04.fc7d32e
ARG VERUS_SHA256=7915e607c2de97a26fdb82ebd47d876fa48dba2693fed9c9a1cd1a707bfa01f8
ARG CODEQL_BUNDLE=codeql-bundle-v2.27.1
ARG CODEQL_SHA256=1ec99cfa9420f04c2330784b4ddb8363a0dd67c3e4471cd93963c50e6c433717
ARG NODE_VERSION=v22.23.3
ARG NODE_SHA256=df450af89261115ef9f9e3830c3eeb2cc9213b63c720b1af623cb5dcbe2e02de
ARG CLAUDE_CODE_VERSION=2.1.289

# Optional extra CA for building behind a TLS-intercepting proxy (empty by default).
COPY extra-ca.crt /usr/local/share/ca-certificates/extra-ca.crt
RUN update-ca-certificates
ENV NODE_EXTRA_CA_CERTS=/etc/ssl/certs/ca-certificates.crt

ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:/opt/verus:/opt/codeql:/opt/node/bin:$PATH

# Rust toolchain and cargo-deny.
RUN curl -fsSL -o /tmp/rustup-init https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init \
    && chmod +x /tmp/rustup-init \
    && /tmp/rustup-init -y --profile minimal --default-toolchain ${RUST_TOOLCHAIN} --component clippy,rustfmt,rust-src \
    && rm /tmp/rustup-init \
    && cargo install --locked cargo-deny --version ${CARGO_DENY_VERSION} \
    && rm -rf /opt/cargo/registry/cache /opt/cargo/registry/src \
    && chmod -R a+rwX /opt/rustup /opt/cargo

COPY vendor/ /tmp/vendor/

# Verus (includes Z3).
RUN set -e; \
    if [ -f /tmp/vendor/verus.zip ]; then cp /tmp/vendor/verus.zip /tmp/verus.zip; \
    else curl -fsSL -o /tmp/verus.zip "https://github.com/verus-lang/verus/releases/download/release/rolling/${VERUS_VERSION}/verus-${VERUS_VERSION}-x86-linux.zip"; fi; \
    echo "${VERUS_SHA256}  /tmp/verus.zip" | sha256sum -c -; \
    unzip -q /tmp/verus.zip -d /tmp/verus && mv /tmp/verus/verus-x86-linux /opt/verus && rm -rf /tmp/verus /tmp/verus.zip

# CodeQL bundle (zstd archive; decompressed with the Python zstandard module).
RUN set -e; \
    if [ -f /tmp/vendor/codeql.tar.zst ]; then cp /tmp/vendor/codeql.tar.zst /tmp/codeql.tar.zst; \
    else curl -fsSL -o /tmp/codeql.tar.zst "https://github.com/github/codeql-action/releases/download/${CODEQL_BUNDLE}/codeql-bundle-linux64.tar.zst"; fi; \
    echo "${CODEQL_SHA256}  /tmp/codeql.tar.zst" | sha256sum -c -; \
    python3 -m pip install --no-cache-dir zstandard==0.23.0; \
    python3 -c "import zstandard, tarfile; tarfile.open(fileobj=zstandard.ZstdDecompressor().stream_reader(open('/tmp/codeql.tar.zst','rb')), mode='r|').extractall('/opt', filter='tar')"; \
    rm /tmp/codeql.tar.zst

# Node.js and the Claude Code CLI (agents for steps 1 and 2).
RUN set -e; \
    curl -fsSL -o /tmp/node.tar.xz "https://nodejs.org/dist/${NODE_VERSION}/node-${NODE_VERSION}-linux-x64.tar.xz"; \
    echo "${NODE_SHA256}  /tmp/node.tar.xz" | sha256sum -c -; \
    mkdir -p /opt/node && tar -xJf /tmp/node.tar.xz -C /opt/node --strip-components=1 && rm /tmp/node.tar.xz; \
    npm install -g "@anthropic-ai/claude-code@${CLAUDE_CODE_VERSION}"; \
    rm -rf /tmp/vendor

# Non-root user for working in the container.
RUN useradd -m -s /bin/bash dev
USER dev
