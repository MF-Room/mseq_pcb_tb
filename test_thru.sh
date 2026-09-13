#!/usr/bin/env bash
# Tests MIDI THRU: host sends COUNT MIDI messages on output port OUT_PORT into MIDI IN 1,
# the PCB copies them in hardware to MIDI THRU, and the host receives them on input port IN_PORT.
# Both sides compute CRC32-ISO/HDLC. Prints PASS if they match.
# The receive firmware is flashed first so PB3, which shares the IN 1 line with THRU, is an input.
set -euo pipefail
COUNT=1000
OUT_PORT=${OUT_PORT:-0}
IN_PORT=${IN_PORT:-1}

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FW_OUT=$(mktemp /tmp/fw_out.XXXXXX)
HOST_OUT=$(mktemp /tmp/host_out.XXXXXX)
FW_PID=""
HOST_PID=""

cleanup() {
    [[ -n "$FW_PID" ]]   && kill "$FW_PID"   2>/dev/null || true
    [[ -n "$HOST_PID" ]] && kill "$HOST_PID" 2>/dev/null || true
    [[ -n "$FW_PID" ]]   && wait "$FW_PID"   2>/dev/null || true
    [[ -n "$HOST_PID" ]] && wait "$HOST_PID" 2>/dev/null || true
    rm -f "$FW_OUT" "$HOST_OUT"
}
trap cleanup EXIT

echo "=== Building firmware (receive mode) and midi-tester ==="
cd "$REPO/firmware"
LOG_LEVEL=info cargo build --release 2>&1
cd "$REPO/midi-tester"
cargo build 2>&1

echo "=== Flashing firmware ==="
cd "$REPO/firmware"
# The timeout covers IN 1 receiving nothing, in which case the firmware never stops
LOG_LEVEL=info timeout 60 probe-rs run --chip STM32F413CHUx \
    target/thumbv7em-none-eabihf/release/firmware \
    > "$FW_OUT" 2>/dev/null &
FW_PID=$!
sleep 8   # covers flash + probe init

echo "=== Starting host receiver on THRU (port $IN_PORT) ==="
cd "$REPO/midi-tester"
# Stops on its own after watchdog_ms (config.toml), which covers COUNT x interval_ms
cargo run -- receive --port "$IN_PORT" > "$HOST_OUT" 2>&1 &
HOST_PID=$!
sleep 1

echo "=== Sending MIDI messages into IN 1 (port $OUT_PORT, count $COUNT) ==="
SENT_CRC=$(cargo run -- send --count "$COUNT" --port "$OUT_PORT" 2>/dev/null | grep -oE '0x[0-9A-Fa-f]{8}' || true)

echo "=== Waiting for the host receiver watchdog ==="
wait "$HOST_PID" || true
HOST_PID=""
wait "$FW_PID" || true
FW_PID=""

THRU_CRC=$(grep -oE '0x[0-9A-Fa-f]{8}' "$HOST_OUT" | tail -1 || true)

echo ""
echo "Sent CRC : ${SENT_CRC:-NOT FOUND}"
echo "THRU CRC : ${THRU_CRC:-NOT FOUND}"
echo ""

if [[ -n "$SENT_CRC" && "$SENT_CRC" == "$THRU_CRC" ]]; then
    echo "PASS"
else
    echo "FAIL"
    exit 1
fi
