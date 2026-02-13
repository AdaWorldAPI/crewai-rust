# Multi-stage build for crewai-rust server
# ============================================================================

# Stage 1: Build
FROM rust:1.83-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for layer caching
COPY Cargo.toml Cargo.lock* ./

# Create dummy source to cache dependency builds
RUN mkdir -p src/bin && \
    echo "pub fn main() {}" > src/bin/server.rs && \
    echo "pub fn lib() {}" > src/lib.rs && \
    cargo build --release --bin server 2>/dev/null || true

# Copy actual source
COPY src/ src/

# Build the real binary
RUN cargo build --release --bin server

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary
COPY --from=builder /app/target/release/server /app/server

# Default environment
ENV PORT=8080
ENV RUST_LOG=info,crewai=debug

EXPOSE 8080

ENTRYPOINT ["/app/server"]
