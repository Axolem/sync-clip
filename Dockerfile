# Sync Clip encrypted relay — ciphertext-only WebSocket server.
# Build from repo root: docker build -t sync-clip-relay .

FROM rust:1-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/clip-engine ./crates/clip-engine
COPY crates/clip-ffi ./crates/clip-ffi
COPY crates/relay ./crates/relay

# Workspace members must be present for Cargo; only the relay binary is produced.
RUN cargo build --release -p relay

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --create-home --uid 10001 relay
COPY --from=builder /app/target/release/relay /usr/local/bin/relay

USER relay
EXPOSE 7120

# Bind all interfaces inside the container (Shells reach the host/port mapping).
ENV RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/relay"]
CMD ["--bind", "0.0.0.0:7120", "--ttl-secs", "900"]
