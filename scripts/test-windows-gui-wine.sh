#!/usr/bin/env bash
set -euo pipefail

TARGET="${TARGET:-x86_64-pc-windows-gnu}"
EXE="${EXE:-target/$TARGET/debug/glide-gui.exe}"
LOG="${LOG:-/tmp/glide-gui-wine.log}"

if ! command -v wine >/dev/null 2>&1; then
    echo "Missing required tool: wine" >&2
    exit 1
fi

if [[ ! -x "$EXE" ]]; then
    cargo build --package glide-gui --target "$TARGET"
fi

rm -f "$LOG"
WINEDEBUG="${WINEDEBUG:--all}" \
GLIDE_GUI_LOG="Z:\\tmp\\$(basename "$LOG")" \
    wine "$EXE" --smoke

if [[ ! -s "$LOG" ]]; then
    echo "Missing Wine diagnostics log: $LOG" >&2
    exit 1
fi

echo "--- diagnostics ---"
cat "$LOG"
