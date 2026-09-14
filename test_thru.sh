#!/usr/bin/env bash
# Tests MIDI THRU: host sends COUNT MIDI messages into MIDI IN 1, the PCB copies them in hardware
# to MIDI THRU, and the host receives them from there. Both sides compute CRC32-ISO/HDLC.
# Prints PASS if they match. The host ports wired to IN 1 and THRU come from midi_ports.conf.
# The receive firmware is flashed first so PB3, which shares the IN 1 line with THRU, is an input.
set -euo pipefail
COUNT=1000

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Host MIDI ports, written by ./setup_midi_ports.sh
if [[ ! -f "$REPO/midi_ports.conf" ]]; then
    echo "$REPO/midi_ports.conf not found: run ./setup_midi_ports.sh to select the host MIDI ports" >&2
    exit 2
fi
# shellcheck source=/dev/null
source "$REPO/midi_ports.conf"
: "${MIDI_IN1:?not set in midi_ports.conf, rerun ./setup_midi_ports.sh}"
: "${MIDI_THRU:?not set in midi_ports.conf, rerun ./setup_midi_ports.sh}"

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
LOG_LEVEL=info "$REPO/with_timeout.sh" 60 probe-rs run --chip STM32F413CHUx \
    target/thumbv7em-none-eabihf/release/firmware \
    > "$FW_OUT" 2>/dev/null &
FW_PID=$!
sleep 8   # covers flash + probe init

echo "=== Starting host receiver on MIDI THRU (host port '$MIDI_THRU') ==="
cd "$REPO/midi-tester"
# Stops on its own after watchdog_ms (config.toml), which covers COUNT x interval_ms
cargo run -- receive --port "$MIDI_THRU" > "$HOST_OUT" 2>&1 &
HOST_PID=$!
sleep 1

echo "=== Sending MIDI messages into MIDI IN 1 (host port '$MIDI_IN1', count $COUNT) ==="
SENT_CRC=$(cargo run -- send --count "$COUNT" --port "$MIDI_IN1" 2>/dev/null | grep -oE '0x[0-9A-Fa-f]{8}' || true)

echo "=== Waiting for the host receiver watchdog ==="
wait "$HOST_PID" || true
HOST_PID=""
wait "$FW_PID" || true
FW_PID=""

THRU_CRC=$(grep "CRC32" "$HOST_OUT" | grep -oE '0x[0-9A-Fa-f]{8}' | tail -1 || true)

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
