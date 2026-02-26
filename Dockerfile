# =============================================================================
# crewai-rust Full Stack — Docker Build (Vendor Import)
# =============================================================================
# Builds crewai-rust with ladybug-rs, n8n-rs, and rustynum vendored in as
# a single binary.  This replaces the separate adarail_mcp router — all
# barrier, consciousness, and MCP endpoints live in one process.
#
# The barrier stack REST endpoints (/barrier/*) replace the old router's
# /mcp/feel, /mcp/desire, /mcp/hydrate, /mcp/dehydrate pattern.
#
# BUILD:
#   docker build -t crewai-stack:latest .
#
# RUN:
#   docker run -p 8080:8080 crewai-stack:latest
#
# RAILWAY:
#   Set PORT env var (Railway injects it automatically)
#
# ENDPOINTS:
#   GET  /health                    — liveness
#   POST /execute                   — crew.* step delegation
#   POST /chat                      — substrate-driven chat
#   POST /barrier/check-outbound    — 4-layer barrier (outbound)
#   POST /barrier/check-inbound     — 4-layer barrier (inbound)
#   GET  /barrier/topology          — triune facet intensities
#   POST /barrier/feedback          — success/failure feedback
#   GET  /barrier/stats             — markov barrier stats
#   GET  /modules                   — active modules
#   POST /modules/:id/gate-check    — cognitive gate check
# =============================================================================

# =============================================================================
# STAGE 1: Builder
# =============================================================================
FROM rust:1.93-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev cmake protobuf-compiler git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# --- Clone all vendor repos ---
RUN git clone --depth 1 https://github.com/AdaWorldAPI/ladybug-rs vendor/ladybug-rs && \
    git clone --depth 1 https://github.com/AdaWorldAPI/n8n-rs vendor/n8n-rs && \
    git clone --depth 1 https://github.com/AdaWorldAPI/rustynum vendor/rustynum

# --- Copy crewai-rust source ---
COPY . .

# --- Activate vendor deps in Cargo.toml ---
# Uncomment the vendor path dependencies
RUN sed -i 's/^# ladybug-vendor = /ladybug-vendor = /' Cargo.toml && \
    sed -i 's/^# n8n-core = /n8n-core = /' Cargo.toml && \
    sed -i 's/^# n8n-workflow = /n8n-workflow = /' Cargo.toml && \
    sed -i 's/^# n8n-hamming = /n8n-hamming = /' Cargo.toml && \
    sed -i 's/^# rustynum-rs = /rustynum-rs = /' Cargo.toml && \
    sed -i 's/^# vendor-ladybug = /vendor-ladybug = /' Cargo.toml && \
    sed -i 's/^# vendor-n8n = /vendor-n8n = /' Cargo.toml && \
    sed -i 's/^# vendor-rustynum = /vendor-rustynum = /' Cargo.toml && \
    sed -i 's/^# full = /full = /' Cargo.toml

# --- Resolve ladybug-contract path for vendored layout ---
# In Docker, ladybug-contract is at vendor/ladybug-rs/crates/ladybug-contract
RUN sed -i 's|path = "../ladybug-rs/crates/ladybug-contract"|path = "vendor/ladybug-rs/crates/ladybug-contract"|' Cargo.toml

# --- Build optimized binary ---
ARG FEATURES="ladybug"

# AVX-512 binary (Hetzner, AWS c7i, etc.)
RUN RUSTFLAGS="-C target-cpu=x86-64-v4 -C link-arg=-s" \
    cargo build --release --bin server --features "$FEATURES" && \
    cp target/release/server /build/crewai-avx512 && \
    cargo clean -p crewai

# AVX-2 binary (most cloud VMs)
RUN RUSTFLAGS="-C target-cpu=x86-64-v3 -C link-arg=-s" \
    cargo build --release --bin server --features "$FEATURES" && \
    cp target/release/server /build/crewai-avx2 && \
    cargo clean -p crewai

# Generic binary (Railway, any x86-64)
RUN RUSTFLAGS="-C link-arg=-s" \
    cargo build --release --bin server --features "$FEATURES" && \
    cp target/release/server /build/crewai-generic

# =============================================================================
# STAGE 2: Runtime
# =============================================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl procps \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -s /bin/bash crewai \
    && mkdir -p /data && chown crewai:crewai /data

COPY --from=builder /build/crewai-avx512 /usr/local/bin/
COPY --from=builder /build/crewai-avx2   /usr/local/bin/
COPY --from=builder /build/crewai-generic /usr/local/bin/

# Runtime SIMD dispatch: detect CPU features and pick optimal binary
COPY <<'ENTRY' /usr/local/bin/crewai-start
#!/bin/sh
set -e
if grep -q "avx512f" /proc/cpuinfo 2>/dev/null; then
  BIN=crewai-avx512; LVL=AVX-512
elif grep -q "avx2" /proc/cpuinfo 2>/dev/null; then
  BIN=crewai-avx2; LVL=AVX-2
else
  BIN=crewai-generic; LVL=Generic
fi
echo "[crewai-stack] SIMD: ${LVL} -> ${BIN}"
echo "[crewai-stack] Barrier stack: 4-layer (NARS + Markov + Triune + MUL)"
echo "[crewai-stack] Endpoints: /health /barrier/* /execute /chat /modules"
exec "/usr/local/bin/${BIN}" "$@"
ENTRY
RUN chmod +x /usr/local/bin/crewai-start

USER crewai
WORKDIR /home/crewai

# Railway injects PORT automatically; default to 8080
ENV PORT=8080
ENV RUST_LOG=info,crewai=debug

EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:${PORT}/health || exit 1

ENTRYPOINT ["crewai-start"]
