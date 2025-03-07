# Use an official Rust base image (Debian-based)
FROM rust:1.85-slim AS builder

# Install build essentials and dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/canggu

# Copy project files
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build the project with optimizations
RUN cargo build --release

# Final stage: slim runtime image
# Using debian for final stage as its a lighter image compared to rust's official image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from builder
COPY --from=builder /usr/src/canggu/target/release/canggu /usr/local/bin/canggu

# Set entrypoint
ENTRYPOINT ["canggu"]