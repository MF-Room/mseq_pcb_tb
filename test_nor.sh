#!/usr/bin/env bash
# Tests the W25Q16JV NOR flash: firmware checks the JEDEC ID, erases, programs and verifies a sector.
# Prints PASS if the firmware reports "NOR PASS". Destructive for the tested sector.
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

echo "=== Building firmware (nor mode) ==="
cd "$REPO/firmware"
MODE=nor cargo build --release

echo "=== Flashing firmware ==="
# probe-rs run exits when the MCU hits bkpt(); the timeout covers a panic, which never does
timeout 60 probe-rs run --chip STM32F413CHUx \
    target/thumbv7em-none-eabihf/release/firmware \
    > "$FW_OUT" 2>/dev/null &
FW_PID=$!
wait "$FW_PID" || true
FW_PID=""

cat "$FW_OUT"

echo ""
if grep -q "NOR PASS" "$FW_OUT"; then
    echo "PASS"
else
    echo "FAIL"
    exit 1
fi
