#!/usr/bin/env bash
# scripts/test-windows-cli-wine.sh — run Windows CLI smoke tests under Wine.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-lib.sh"

TARGET="${TARGET:-x86_64-pc-windows-gnu}"
EXE="${EXE:-target/$TARGET/debug/glide-cli.exe}"

if ! command -v wine >/dev/null 2>&1; then
    echo "Missing required tool: wine" >&2
    exit 1
fi

if ! command -v winepath >/dev/null 2>&1; then
    echo "Missing required tool: winepath" >&2
    exit 1
fi

export GLIDE_TEST_MANAGED_SERVER="${GLIDE_TEST_MANAGED_SERVER:-1}"
start_managed_server

cargo build --package glide-cli --target "$TARGET"

home_sender="$(mktemp -d)"
home_receiver="$(mktemp -d)"
src_file="$(mktemp)"
out_file="$(mktemp)"
wine_prefix="$(mktemp -d)"

cleanup_windows_cli_wine() {
    rm -rf "$home_sender" "$home_receiver" "$src_file" "$out_file" "$wine_prefix"
}
trap cleanup_windows_cli_wine EXIT

mkdir -p "$home_sender/.config/glide" "$home_receiver/.config/glide"

cat >"$home_sender/.config/glide/config.json" <<EOF
{
  "server_url": "$GLIDE_SERVER",
  "device_id": "33333333-3333-4333-8333-333333333333",
  "device_name": "wine-windows-cli-sender",
  "registration_token": "reg123"
}
EOF

cat >"$home_receiver/.config/glide/config.json" <<EOF
{
  "server_url": "$GLIDE_SERVER",
  "device_id": "44444444-4444-4444-8444-444444444444",
  "device_name": "wine-windows-cli-receiver",
  "registration_token": "reg123"
}
EOF

printf 'windows cli wine payload 中文 emoji 🚀\n' >"$src_file"

src_win="$(winepath -w "$src_file")"
out_win="$(winepath -w "$out_file")"
sender_config_win="$(winepath -w "$home_sender/.config/glide/config.json")"
receiver_config_win="$(winepath -w "$home_receiver/.config/glide/config.json")"

echo "=== Windows CLI Wine Smoke ==="
WINEPREFIX="$wine_prefix" GLIDE_CONFIG_PATH="$sender_config_win" wine "$EXE" --server "$GLIDE_SERVER" copy "wine text smoke"
sleep 1
WINEPREFIX="$wine_prefix" GLIDE_CONFIG_PATH="$receiver_config_win" wine "$EXE" --server "$GLIDE_SERVER" paste | grep -F "wine text smoke" >/dev/null
echo "  OK: text copy/paste"

WINEPREFIX="$wine_prefix" GLIDE_CONFIG_PATH="$sender_config_win" wine "$EXE" --server "$GLIDE_SERVER" copy --file "$src_win"
sleep 1
WINEPREFIX="$wine_prefix" GLIDE_CONFIG_PATH="$receiver_config_win" wine "$EXE" --server "$GLIDE_SERVER" paste --output "$out_win"
cmp "$src_file" "$out_file"
echo "  OK: file payload upload/download"
