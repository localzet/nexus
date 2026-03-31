# Multi-stage build for NexusDB

# ========== BUILDER ==========
FROM rust:1.70 as builder

WORKDIR /build

# Copy source code
COPY . .

# Build release binary
RUN cargo build --release

# ========== RUNTIME ==========
FROM debian:bookworm-slim

LABEL org.opencontainers.image.title="NexusDB"
LABEL org.opencontainers.image.description="Multi-Model Database Engine"
LABEL org.opencontainers.image.source="https://github.com/localzet/nexus"
LABEL org.opencontainers.image.licenses="AGPL-3.0"

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create nexus user
RUN useradd -m -s /bin/bash nexus

# Copy binary from builder
COPY --from=builder /build/target/release/nexus /usr/local/bin/nexus
RUN chmod +x /usr/local/bin/nexus

# Create directories
RUN mkdir -p /etc/nexus /var/lib/nexus && \
    chown -R nexus:nexus /etc/nexus /var/lib/nexus

# Copy default config
COPY --chown=nexus:nexus ./config/nexus.toml.example /etc/nexus/nexus.toml

WORKDIR /var/lib/nexus

USER nexus

# Ports
# NQL Protocol (TCP) — unified interface
EXPOSE 5433

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Start NexusDB
CMD ["nexus"]
