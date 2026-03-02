# ─── Stage 1: Builder ───────────────────────────────────────────────────────
FROM rust:latest AS builder

# Install OpenSSL dependencies required by reqwest
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Cache dependencies before copying source
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/dollar_brl*

# Copy real source and build
COPY src ./src
RUN cargo build --release

# ─── Stage 2: Runtime ───────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy only the compiled binary from builder
COPY --from=builder /app/target/release/dollar-brl .

CMD ["./dollar-brl"]