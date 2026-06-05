#!/usr/bin/env bash
# Linux E2E wrapper that runs against an isolated managed Glide server by default.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test-lib.sh"

export GLIDE_TEST_MANAGED_SERVER="${GLIDE_TEST_MANAGED_SERVER:-1}"
start_managed_server

python3 "$SCRIPT_DIR/test-e2e.py"
