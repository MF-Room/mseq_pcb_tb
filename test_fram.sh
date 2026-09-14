#!/usr/bin/env bash
# Tests the MB85RS256B FRAM: firmware writes and verifies the whole memory and checks the address lines.
# Prints PASS if the firmware reports "FRAM PASS". Destructive for the whole FRAM.
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

echo "=== Building firmware (fram mode) ==="
cd "$REPO/firmware"
MODE=fram cargo build --release

echo "=== Flashing firmware ==="
# probe-rs run exits when the MCU hits bkpt(); the timeout covers a panic, which never does
"$REPO/with_timeout.sh" 60 probe-rs run --chip STM32F413CHUx \
    target/thumbv7em-none-eabihf/release/firmware \
    > "$FW_OUT" 2>/dev/null &
FW_PID=$!
wait "$FW_PID" || true
FW_PID=""

cat "$FW_OUT"

echo ""
if grep -q "FRAM PASS" "$FW_OUT"; then
    echo "PASS"
else
    echo "FAIL"
    exit 1
fi
