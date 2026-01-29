FROM rust:1.80-slim-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin ferrotunnel-server --bin ferrotunnel-client

FROM debian:bookworm-slim AS runtime
WORKDIR /app
# Install OpenSSL/CA certs
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r ferrotunnel && useradd -r -g ferrotunnel ferrotunnel

COPY --from=builder /app/target/release/ferrotunnel-server /usr/local/bin/
COPY --from=builder /app/target/release/ferrotunnel-client /usr/local/bin/

USER ferrotunnel
EXPOSE 8080 9090 4040

# Default to server, but allows easy override for client
CMD ["ferrotunnel-server", "--bind", "0.0.0.0:7835"]
