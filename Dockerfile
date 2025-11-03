# Build stage
FROM rust:1.91-bookworm AS builder

WORKDIR /build

# Copy the entire project structure including local dependencies
# Note: Build context must include parent directory for meshtastic-rust dependency
COPY mesh-jawn/ ./mesh-jawn/
COPY meshtastic-rust/ ./meshtastic-rust/

WORKDIR /build/mesh-jawn

# Build the release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install CA certificates for HTTPS (if needed) and clean up
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /build/mesh-jawn/target/release/mesh-jawn /usr/local/bin/mesh-jawn

# Run as non-root user
RUN useradd -m -u 1000 meshjawn
USER meshjawn

ENTRYPOINT ["/usr/local/bin/mesh-jawn"]
CMD ["--help"]
