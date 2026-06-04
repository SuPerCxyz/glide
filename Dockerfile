FROM rust:1.87-slim AS builder

WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
RUN cargo build --release --package glide-server --package glide-cli

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

ENV GLIDE_DATA_DIR=/data
ENV GLIDE_PUBLIC_URL=http://localhost:8080
ENV GLIDE_INPUT_RELAY_ENABLED=false

COPY --from=builder /app/target/release/glide-server /usr/local/bin/glide-server
COPY --from=builder /app/target/release/glide-cli /usr/local/bin/glide-cli

EXPOSE 8080

CMD ["sh", "-c", "mkdir -p $GLIDE_DATA_DIR && glide-server"]
