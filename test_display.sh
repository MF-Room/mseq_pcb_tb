#!/usr/bin/env bash
# Tests the LCD display: firmware shows a random number, user reads it and types it here.
# Prints PASS if the input matches.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FW_OUT=$(mktemp /tmp/fw_out.XXXXXX)
FW_PID=""

cleanup() {
    [[ -n "$FW_PID" ]] && kill "$FW_PID" 2>/dev/null || true
    [[ -n "$FW_PID" ]] && wait "$FW_PID" 2>/dev/null || true
    rm -f "$FW_OUT"
}
trap cleanup EXIT

echo "=== Building firmware (display mode) ==="
cd "$REPO/firmware"
MODE=display cargo build --release

echo "=== Flashing firmware ==="
MODE=display probe-rs run --chip STM32F413CHUx \
    target/thumbv7em-none-eabihf/release/firmware \
    > "$FW_OUT" 2>/dev/null &
FW_PID=$!
wait "$FW_PID" || true
FW_PID=""

EXPECTED=$(grep -oE 'Number: [0-9]+' "$FW_OUT" | grep -oE '[0-9]+' || true)

echo ""
read -rp "Enter the number shown on the LCD: " USER_INPUT

echo ""
if [[ "$USER_INPUT" == "$EXPECTED" ]]; then
    echo "PASS"
else
    echo "FAIL (expected: ${EXPECTED:-NOT FOUND})"
    exit 1
fi
