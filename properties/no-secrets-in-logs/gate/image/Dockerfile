# Pinned gate image. Build with ./build.sh (fetches and checks the pinned tool
# archives, stages the package, builds, prints the image ID).
# Run:
#   docker run --rm --network none \
#     -v <run-dir>:/work/run:ro -v <triage-dir>:/work/triage:ro -v <report-dir>:/work/report \
#     nosecrets-gate:v0.2 /work/run --classification-sha256 <hash> \
#     [--triage /work/triage/<file>] --report /work/report
#
# Base image pinned by digest (python:3.11-trixie via the Docker Hub mirror on
# mirror.gcr.io; it already contains git, gcc, curl and unzip; Verus needs
# glibc >= 2.39, trixie has 2.41). The image ID
# printed by build.sh is what verdicts must reference.
ARG BASE=mirror.gcr.io/library/python@sha256:27e044f7e01fea05c1760324d58fc5360a0767b9ef098e74ddaf8c70b8f46d26
FROM ${BASE}

ARG RUST_TOOLCHAIN=1.98.1
ARG CARGO_DENY_VERSION=0.18.9

# Empty unless the image is built behind a TLS-intercepting proxy (build.sh).
COPY extra-ca.crt /usr/local/share/ca-certificates/extra-ca.crt
RUN update-ca-certificates

ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo PATH=/opt/cargo/bin:/opt/verus:/opt/codeql:$PATH
RUN curl -fsSL -o /tmp/rustup-init https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init \
    && chmod +x /tmp/rustup-init \
    && /tmp/rustup-init -y --profile minimal --default-toolchain ${RUST_TOOLCHAIN} --component clippy,rustfmt,rust-src \
    && rm /tmp/rustup-init \
    && cargo install --locked cargo-deny --version ${CARGO_DENY_VERSION}

COPY verus.zip /tmp/verus.zip
RUN unzip -q /tmp/verus.zip -d /tmp/verus && mv /tmp/verus/verus-x86-linux /opt/verus && rm -rf /tmp/verus /tmp/verus.zip
# Unpacked on the host by build.sh (the base image has no zstd).
COPY codeql /opt/codeql

# The package itself (tools, codegen, lib, template, codeql, gate, examples).
COPY package /opt/package

# Warm the cargo cache so the gate runs with --network none and verdicts do
# not depend on the network.
RUN echo '# warmup' > /tmp/req.md \
    && /opt/package/tools/new-project.sh /tmp/warm /tmp/req.md \
    && cp /opt/package/examples/demo.classification.toml /tmp/warm/classification.toml \
    && /opt/package/tools/freeze.sh /tmp/warm \
    && echo "fn main() {}" > /tmp/warm/app/src/main.rs \
    && cd /tmp/warm && cargo fetch --locked \
    # The CodeQL Rust extractor also loads the rust-src workspace, whose
    # crates must be in the cache too; check that loading worked.
    && codeql database create /tmp/warmdb --language=rust --source-root=/tmp/warm > /tmp/warmdb.log 2>&1 \
    && ! grep -q "failed to load workspace" /tmp/warmdb.log \
    && rm -rf /tmp/warm /tmp/warmdb /tmp/warmdb.log /tmp/req.md
ENV CARGO_NET_OFFLINE=true

ENTRYPOINT ["python3", "/opt/package/gate/gate.py"]
