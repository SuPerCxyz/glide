FROM rust:1.79-slim AS builder

WORKDIR /app
COPY . .
RUN cargo build --release --package glide-server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m glide
USER glide

ENV GLIDE_DATA_DIR=/data
ENV GLIDE_PUBLIC_URL=http://localhost:8080
ENV GLIDE_INPUT_RELAY_ENABLED=false

COPY --from=builder /app/target/release/glide-server /usr/local/bin/glide-server

RUN mkdir -p /data && chown glide:glide /data

EXPOSE 8080

CMD ["glide-server"]
