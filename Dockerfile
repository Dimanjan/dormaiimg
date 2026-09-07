# ==============================================================================
# Dharke Clothing Matcher - Ultra-Low Resource Fast-Deploy Container
# Standing RAM: ~10 MB | Image Size: ~35 MB | Build Time: ~5 seconds
# ==============================================================================

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY bin/rust_matcher /app/rust_matcher
RUN chmod +x /app/rust_matcher

ENV PORT=8000
EXPOSE 8000

CMD ["/app/rust_matcher"]
