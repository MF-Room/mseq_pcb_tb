#!/usr/bin/env bash
# Tests MCU receive path: host sends COUNT MIDI messages into the MIDI input under test, MCU receives,
# both compute CRC32-ISO/HDLC. Prints PASS if they match.
# Usage: ./test_receive.sh [1|2]   MIDI input under test (default 1). The host port wired to it comes from midi_ports.conf.
set -euo pipefail
command -v timeout >/dev/null || { echo "timeout not found: install GNU coreutils (macOS: brew install coreutils)" >&2; exit 2; }
COUNT=3000
INPUT=${1:-1}

case "$INPUT" in
    1) MODE="" ;;
    2) MODE="receive2" ;;
    *) echo "Usage: $0 [1|2]"; exit 2 ;;
esac

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Host MIDI ports, written by ./setup_midi_ports.sh
if [[ ! -f "$REPO/midi_ports.conf" ]]; then
    echo "$REPO/midi_ports.conf not found: run ./setup_midi_ports.sh to select the host MIDI ports" >&2
    exit 2
fi
# shellcheck source=/dev/null
source "$REPO/midi_ports.conf"
case "$INPUT" in
    1) HOST_PORT=${MIDI_IN1:-} ;;
    2) HOST_PORT=${MIDI_IN2:-} ;;
esac
: "${HOST_PORT:?MIDI_IN$INPUT not set in midi_ports.conf, rerun ./setup_midi_ports.sh}"

FW_OUT=$(mktemp /tmp/fw_out.XXXXXX)
FW_PID=""

cleanup() {
    [[ -n "$FW_PID" ]] && kill "$FW_PID" 2>/dev/null || true
    [[ -n "$FW_PID" ]] && wait "$FW_PID" 2>/dev/null || true
    rm -f "$FW_OUT"
}
trap cleanup EXIT

echo "=== Building firmware (receive mode, MIDI IN $INPUT) and midi-tester ==="
cd "$REPO/firmware"
MODE=$MODE LOG_LEVEL=info cargo build --release 2>&1
cd "$REPO/midi-tester"
cargo build --release 2>&1

echo "=== Flashing firmware ==="
cd "$REPO/firmware"
# The MCU stops itself 2 s after the last byte; the limit covers nothing arriving, in which case it never stops
LOG_LEVEL=info timeout 60 probe-rs run --chip STM32F413CHUx \
    target/thumbv7em-none-eabihf/release/firmware \
    > "$FW_OUT" 2>/dev/null &
FW_PID=$!
sleep 8   # covers flash + probe init

echo ""
echo "=== Sending MIDI messages into MIDI IN $INPUT (host port '$HOST_PORT', count $COUNT) ==="
cd "$REPO/midi-tester"
HOST_CRC=$(cargo run --release -- send --count "$COUNT" --port "$HOST_PORT" 2>/dev/null | grep -oE '0x[0-9A-Fa-f]{8}' || true)

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
