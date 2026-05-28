#!/usr/bin/env bash
# Tests MCU send path: MCU sends COUNT MIDI messages, host receives on port PORT,
# both compute CRC32-ISO/HDLC. Prints PASS if they match.
# NOTE: currently expected to fail due to a PCB hardware issue on the TX path.
set -euo pipefail
COUNT=1000
PORT=1

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FW_OUT=$(mktemp /tmp/fw_out.XXXXXX)
HOST_OUT=$(mktemp /tmp/host_out.XXXXXX)
FW_PID=""
HOST_PID=""

cleanup() {
    pkill -f "probe-rs run --chip STM32F413CHUx" 2>/dev/null || true
    [[ -n "$FW_PID" ]]   && kill "$FW_PID"   2>/dev/null || true
    [[ -n "$HOST_PID" ]] && kill "$HOST_PID" 2>/dev/null || true
    [[ -n "$FW_PID" ]]   && wait "$FW_PID"   2>/dev/null || true
    [[ -n "$HOST_PID" ]] && wait "$HOST_PID" 2>/dev/null || true
    rm -f "$FW_OUT" "$HOST_OUT"
}
trap cleanup EXIT

echo "=== Building firmware (send mode, COUNT=$COUNT) ==="
cd "$REPO/firmware"
COUNT=$COUNT MODE=send cargo build --release

echo "=== Starting host receiver (port $PORT) ==="
cd "$REPO/host"
cargo run -- receive --port "$PORT" > "$HOST_OUT" 2>&1 &
HOST_PID=$!

echo "=== Flashing and attaching RTT (send mode) ==="
cd "$REPO/firmware"
COUNT=$COUNT MODE=send probe-rs run --chip STM32F413CHUx \
    target/thumbv7em-none-eabihf/release/firmware \
    > >(tee "$FW_OUT") 2>&1 &
FW_PID=$!

echo "=== Waiting for MCU to finish sending and probe-rs to exit ==="
wait "$FW_PID" || true
FW_PID=""

# Give host a moment to flush the last messages, then stop it
sleep 2
kill "$HOST_PID" 2>/dev/null || true
wait "$HOST_PID" 2>/dev/null || true
HOST_PID=""

FW_CRC=$(grep -oE '0x[0-9A-Fa-f]{8}' "$FW_OUT" | tail -1 || true)
HOST_CRC=$(grep -oE '0x[0-9A-Fa-f]{8}' "$HOST_OUT" | tail -1 || true)

echo ""
echo "MCU  CRC : ${FW_CRC:-NOT FOUND}"
echo "Host CRC : ${HOST_CRC:-NOT FOUND}"
echo ""

if [[ -n "$FW_CRC" && "$FW_CRC" == "$HOST_CRC" ]]; then
    echo "PASS"
else
    echo "FAIL (expected — PCB TX path issue)"
    exit 1
fi
