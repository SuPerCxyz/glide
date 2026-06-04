FROM rust:1.87-slim AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Layer 1: Copy only dependency manifests (cached when deps don't change)
COPY Cargo.toml Cargo.lock ./
COPY crates/glide-core/Cargo.toml crates/glide-core/Cargo.toml
COPY crates/glide-server/Cargo.toml crates/glide-server/Cargo.toml
COPY crates/glide-cli/Cargo.toml crates/glide-cli/Cargo.toml
COPY crates/glide-desktop/Cargo.toml crates/glide-desktop/Cargo.toml

# Create dummy source files so cargo can resolve deps
RUN mkdir -p crates/glide-core/src crates/glide-server/src crates/glide-cli/src crates/glide-desktop/src crates/glide-desktop/src/linux_backends crates/glide-desktop/tests crates/glide-server/tests crates/glide-core/tests crates/glide-server/static && \
    echo "pub fn dummy() {}" > crates/glide-core/src/lib.rs && \
    echo "pub fn main() {}" > crates/glide-server/src/main.rs && \
    echo "pub fn dummy() {}" > crates/glide-server/src/lib.rs && \
    echo "pub fn main() {}" > crates/glide-cli/src/main.rs && \
    echo "pub mod clipboard_adapter; pub mod input_adapter; pub mod policy_ui; pub mod linux_backends; pub mod windows_clipboard; pub mod lan_sync;" > crates/glide-desktop/src/lib.rs && \
    echo "" > crates/glide-desktop/src/clipboard_adapter.rs && \
    echo "" > crates/glide-desktop/src/input_adapter.rs && \
    echo "" > crates/glide-desktop/src/policy_ui.rs && \
    echo "pub mod headless; pub mod wayland; pub mod x11;" > crates/glide-desktop/src/linux_backends.rs && \
    echo "" > crates/glide-desktop/src/linux_backends/headless.rs && \
    echo "" > crates/glide-desktop/src/linux_backends/wayland.rs && \
    echo "" > crates/glide-desktop/src/linux_backends/x11.rs && \
    echo "" > crates/glide-desktop/src/windows_clipboard.rs && \
    echo "" > crates/glide-desktop/src/lan_sync.rs && \
    echo "" > crates/glide-desktop/tests/lan_sync_tests.rs && \
    echo "" > crates/glide-server/tests/server_tests.rs && \
    echo "" > crates/glide-core/tests/core_types.rs && \
    echo "<html></html>" > crates/glide-server/static/index.html

# Build deps only (cached unless Cargo.toml/Cargo.lock change)
RUN cargo build --release --package glide-server --package glide-cli 2>&1 || true

# Layer 2: Copy real source and rebuild (only recompiles our code)
COPY crates/ crates/

# Touch dummy files to force rebuild of our crates only
RUN touch crates/glide-core/src/lib.rs crates/glide-server/src/lib.rs crates/glide-cli/src/main.rs

RUN cargo build --release --package glide-server --package glide-cli

# --- Runtime image ---
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
