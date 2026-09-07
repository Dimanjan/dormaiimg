# ==============================================================================
# Dharke Clothing Matcher - Ultra-Low Resource Headless Docker Container
# Standing RAM: ~15 MB | Image Size: ~30 MB
# ==============================================================================

# Build Stage
FROM rust:1.80-slim-bookworm AS builder

WORKDIR /usr/src/clothing-matcher

# Copy source and embedded catalogue data
COPY data/catalogue data/catalogue
COPY rust_matcher rust_matcher

WORKDIR /usr/src/clothing-matcher/rust_matcher
RUN cargo build --release

# Runtime Stage (Ultra-minimal Debian Slim with standard glibc)
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/clothing-matcher/rust_matcher/target/release/rust_matcher /app/rust_matcher

# Non-root user for security
RUN useradd -m -u 1000 appuser && chown -R appuser:appuser /app
USER appuser

ENV PORT=8000
EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=3s CMD curl -f http://localhost:8000/health || exit 1

CMD ["/app/rust_matcher"]
