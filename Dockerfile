# Runtime-only image: binaries are pre-built by CI, just copy them in.
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

ENV GLIDE_DATA_DIR=/data
ENV GLIDE_PUBLIC_URL=http://localhost:8080
ENV GLIDE_INPUT_RELAY_ENABLED=false

COPY target/release/glide-server /usr/local/bin/glide-server
COPY target/release/glide-cli /usr/local/bin/glide-cli
COPY crates/glide-server/static/index.html /usr/local/share/glide/index.html

EXPOSE 8080

CMD ["sh", "-c", "mkdir -p $GLIDE_DATA_DIR && glide-server"]
