FROM rust:1-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/coherence-core-db-bootstrap
COPY . .

RUN cargo install --path . --locked --root /opt/coherence-install

FROM rust:1-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl git bash \
    && curl -fsSL https://github.com/dolthub/dolt/releases/latest/download/install.sh | bash \
    && dolt config --global --add user.email "demo@example.invalid" \
    && dolt config --global --add user.name "Coherence Demo" \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /opt/coherence-install/bin/coherence-bootstrap /usr/local/bin/coherence-bootstrap
COPY scripts/demo-container-entrypoint /usr/local/bin/coherence-demo-entrypoint
COPY scripts/demo-container-init /usr/local/bin/coherence-demo-init
COPY scripts/demo-container-smoke /usr/local/bin/coherence-demo-smoke

ENV COHERENCE_DOLT_DATA_DIR=/var/lib/coherence/dolt \
    COHERENCE_DOLT_RUNTIME_DIR=/tmp/coherence \
    DOLT_SOCKET=/tmp/coherence/dolt.sock \
    DOLT_HOST=127.0.0.1 \
    DOLT_PORT=33306

WORKDIR /workspace
ENTRYPOINT ["coherence-demo-entrypoint"]
CMD ["bash"]
