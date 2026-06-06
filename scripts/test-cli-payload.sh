#!/usr/bin/env bash
# scripts/test-cli-payload.sh — CLI payload upload/download smoke test.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-lib.sh"

export GLIDE_TEST_MANAGED_SERVER="${GLIDE_TEST_MANAGED_SERVER:-1}"
start_managed_server

cargo build --package glide-cli >/dev/null

home_sender="$(mktemp -d)"
home_receiver="$(mktemp -d)"
src_file="$(mktemp)"
out_file="$(mktemp)"

cleanup_payload_test() {
  rm -rf "$home_sender" "$home_receiver" "$src_file" "$out_file"
}
trap cleanup_payload_test EXIT

mkdir -p "$home_sender/.config/glide" "$home_receiver/.config/glide"

cat > "$home_sender/.config/glide/config.json" <<EOF
{
  "server_url": "$GLIDE_SERVER",
  "device_id": "11111111-1111-4111-8111-111111111111",
  "device_name": "cli-payload-sender",
  "registration_token": "reg123"
}
EOF

cat > "$home_receiver/.config/glide/config.json" <<EOF
{
  "server_url": "$GLIDE_SERVER",
  "device_id": "22222222-2222-4222-8222-222222222222",
  "device_name": "cli-payload-receiver",
  "registration_token": "reg123"
}
EOF

printf 'payload upload text 中文 emoji 🚀\n' > "$src_file"

echo "=== CLI Payload Upload/Download Test ==="
HOME="$home_sender" target/debug/glide-cli --server "$GLIDE_SERVER" copy --file "$src_file"
sleep 1
HOME="$home_receiver" target/debug/glide-cli --server "$GLIDE_SERVER" paste --output "$out_file"

cmp "$src_file" "$out_file"
echo "  ✅ CLI file payload upload/download"
