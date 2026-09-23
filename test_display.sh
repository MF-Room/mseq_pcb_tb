#!/usr/bin/env bash
# Tests the OLED display (HS242L03B2C01, SSD1309 on I2C): firmware shows a random number, user reads it
# and types it here. Prints PASS if the input matches.
set -euo pipefail
command -v timeout >/dev/null || { echo "timeout not found: install GNU coreutils (macOS: brew install coreutils)" >&2; exit 2; }

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
# The MCU stops itself once the number is shown; the limit covers a panic, which never does
MODE=display timeout 30 probe-rs run --chip STM32F413CHUx \
    target/thumbv7em-none-eabihf/release/firmware \
    > "$FW_OUT" 2>/dev/null &
FW_PID=$!
wait "$FW_PID" || true
FW_PID=""

EXPECTED=$(grep -oE 'Number: [0-9]+' "$FW_OUT" | grep -oE '[0-9]+' || true)
if [[ -z "$EXPECTED" ]]; then
    cat "$FW_OUT"
    echo ""
    echo "FAIL (firmware did not report a number)"
    exit 1
fi

echo ""
read -rp "Enter the number shown on the display: " USER_INPUT

echo ""
if [[ "$USER_INPUT" == "$EXPECTED" ]]; then
    echo "PASS"
else
    echo "FAIL (expected: ${EXPECTED:-NOT FOUND})"
    exit 1
fi
