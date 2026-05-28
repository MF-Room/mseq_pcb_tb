#!/usr/bin/env bash
# Tests MCU receive path: host sends COUNT MIDI messages on port PORT, MCU receives,
# both compute CRC32-ISO/HDLC. Prints PASS if they match.
set -euo pipefail
COUNT=3000
PORT=0

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FW_OUT=$(mktemp /tmp/fw_out.XXXXXX)
FW_PID=""

cleanup() {
    pkill -f "probe-rs run --chip STM32F413CHUx" 2>/dev/null || true
    [[ -n "$FW_PID" ]] && kill "$FW_PID" 2>/dev/null || true
    [[ -n "$FW_PID" ]] && wait "$FW_PID" 2>/dev/null || true
    rm -f "$FW_OUT"
}
trap cleanup EXIT

echo "=== Building firmware (receive mode, debug logging) ==="
cd "$REPO/firmware"
LOG_LEVEL=info cargo build --release

echo "=== Flashing and attaching RTT (debug output shown below) ==="
# probe-rs run exits on its own when the MCU hits bkpt(); tee keeps output visible
LOG_LEVEL=info probe-rs run --chip STM32F413CHUx \
    target/thumbv7em-none-eabihf/release/firmware \
    > >(tee "$FW_OUT") 2>&1 &
FW_PID=$!
sleep 8   # covers flash + probe init

echo ""
echo "=== Sending MIDI messages (port $PORT, count $COUNT) ==="
cd "$REPO/host"
HOST_CRC=$(cargo run -- send --count "$COUNT" --port "$PORT" 2>/dev/null | grep -oE '0x[0-9A-Fa-f]{8}' || true)

echo "=== Waiting for MCU watchdog to fire and probe-rs to exit ==="
wait "$FW_PID" || true
FW_PID=""

FW_CRC=$(grep "bytes)" "$FW_OUT" | grep -oE '0x[0-9A-Fa-f]{8}' | head -1 || true)

echo ""
echo "Host CRC : ${HOST_CRC:-NOT FOUND}"
echo "MCU  CRC : ${FW_CRC:-NOT FOUND}"
echo ""

if [[ -n "$HOST_CRC" && "$HOST_CRC" == "$FW_CRC" ]]; then
    echo "PASS"
else
    echo "FAIL"
    exit 1
fi
