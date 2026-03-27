# Build stage
FROM rust:1.75-bookworm AS builder

WORKDIR /app
COPY code/Cargo.toml code/Cargo.lock ./
COPY code/src ./src

RUN apt-get update && apt-get install -y libgtk-4-dev libssh2-1-dev pkg-config && \
    cargo build --release

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
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/ilogcat /usr/bin/ilogcat

# Create a non-root user
RUN useradd -m -s /bin/bash ilogcat
USER ilogcat

ENTRYPOINT ["/usr/bin/ilogcat"]