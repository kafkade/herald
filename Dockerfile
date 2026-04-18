# =============================================================================
# Stage 1: Dependency planner (cargo-chef)
# =============================================================================
FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# =============================================================================
# Stage 2: Build the Herald server binary
# =============================================================================
FROM chef AS server-builder

COPY --from=planner /app/recipe.json recipe.json
# Build dependencies only (cached unless Cargo.toml/Cargo.lock change)
RUN cargo chef cook --release --recipe-path recipe.json -p herald-server

# Copy source and build the actual binary
COPY . .
RUN cargo build --release -p herald-server

# =============================================================================
# Stage 3: Build the Leptos/Wasm web frontend
# =============================================================================
FROM rust:1-bookworm AS web-builder

RUN rustup target add wasm32-unknown-unknown \
    && cargo install trunk

WORKDIR /app
COPY . .

RUN cd crates/herald-web && trunk build --release

# =============================================================================
# Stage 4: Minimal runtime image
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash herald

WORKDIR /home/herald

# Copy server binary
COPY --from=server-builder /app/target/release/herald-server /usr/local/bin/herald-server

# Copy web assets
COPY --from=web-builder /app/crates/herald-web/dist ./static

# Create data directory for SQLite
RUN mkdir -p /data && chown herald:herald /data

USER herald

# Configuration
ENV HERALD_PORT=3000 \
    HERALD_DB_PATH=/data/herald.db \
    HERALD_WEB_DIR=/home/herald/static \
    HERALD_LOG_LEVEL=info

EXPOSE 3000

ENTRYPOINT ["herald-server"]
