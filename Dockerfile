# Build stage
FROM rust:1.83-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    libgtk-4-dev \
    libssh2-1-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for better caching
COPY code/Cargo.toml code/Cargo.lock ./

# Create dummy src to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source
COPY code/src ./src

# Build for real
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libgtk-4-1 \
    libssh2-1 \
    libglib2.0-0 \
    libpango-1.0-0 \
    libpangocairo-1.0-0 \
    libcairo2 \
    libgdk-pixbuf-2.0-0 \
    librsvg2-2 \
    libadwaita-1-0 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/ilogcat /usr/bin/ilogcat

ENTRYPOINT ["/usr/bin/ilogcat"]