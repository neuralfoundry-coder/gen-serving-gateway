# Build stage
FROM rust:1.83-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Create dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    mkdir -p src/api src/backend src/config src/gateway src/middleware src/queue src/response && \
    echo "" > src/lib.rs

# Build dependencies only
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src
COPY build.rs ./
COPY proto ./proto
COPY config ./config

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# OCI Labels
LABEL org.opencontainers.image.title="Generative Image Serving Framework"
LABEL org.opencontainers.image.description="Rust-based framework for integrating multiple generative image model backends"
LABEL org.opencontainers.image.source="https://github.com/your-org/generative-img-serving"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.vendor="Generative Image Serving"

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false appuser

# Copy the binary
COPY --from=builder /app/target/release/img-serving /app/img-serving

# Copy config
COPY config ./config

# Create directory for generated images
RUN mkdir -p /app/generated_images && \
    chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

EXPOSE 8080

ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["/app/img-serving"]
