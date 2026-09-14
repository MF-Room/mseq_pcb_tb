#!/usr/bin/env bash
# Tests MCU send path: MCU sends COUNT MIDI messages on MIDI OUT, host receives them,
# both compute CRC32-ISO/HDLC. Prints PASS if they match. The host port wired to MIDI OUT comes from midi_ports.conf.
set -euo pipefail
command -v timeout >/dev/null || { echo "timeout not found: install GNU coreutils (macOS: brew install coreutils)" >&2; exit 2; }
COUNT=1000

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Host MIDI ports, written by ./setup_midi_ports.sh
if [[ ! -f "$REPO/midi_ports.conf" ]]; then
    echo "$REPO/midi_ports.conf not found: run ./setup_midi_ports.sh to select the host MIDI ports" >&2
    exit 2
fi
# shellcheck source=/dev/null
source "$REPO/midi_ports.conf"
: "${MIDI_OUT:?not set in midi_ports.conf, rerun ./setup_midi_ports.sh}"

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

echo "=== Building firmware (send mode, COUNT=$COUNT) and midi-tester ==="
cd "$REPO/firmware"
COUNT=$COUNT MODE=send cargo build --release
cd "$REPO/midi-tester"
cargo build 2>&1

echo "=== Starting host receiver on MIDI OUT (host port '$MIDI_OUT') ==="
cargo run -- receive --port "$MIDI_OUT" > "$HOST_OUT" 2>&1 &
HOST_PID=$!

echo "=== Flashing and attaching RTT (send mode) ==="
cd "$REPO/firmware"
# The MCU stops itself after COUNT messages (about 5 s); the limit covers a panic, which never does
COUNT=$COUNT MODE=send timeout 30 probe-rs run --chip STM32F413CHUx \
    target/thumbv7em-none-eabihf/release/firmware \
    > "$FW_OUT" 2>/dev/null &
FW_PID=$!

echo "=== Waiting for MCU to finish sending and probe-rs to exit ==="
wait "$FW_PID" || true
FW_PID=""

# The receiver prints its CRC when its watchdog_ms (config.toml) deadline passes; killing it earlier loses it
echo "=== Waiting for the host receiver watchdog ==="
wait "$HOST_PID" || true
HOST_PID=""

# The CRC is on the firmware's own log line; probe-rs prints a backtrace with addresses after it
FW_CRC=$(grep "firmware::send" "$FW_OUT" | grep -oE '0x[0-9A-Fa-f]{8}' | head -1 || true)
HOST_CRC=$(grep "CRC32" "$HOST_OUT" | grep -oE '0x[0-9A-Fa-f]{8}' | tail -1 || true)

echo ""
echo "MCU  CRC : ${FW_CRC:-NOT FOUND}"
echo "Host CRC : ${HOST_CRC:-NOT FOUND}"
echo ""

if [[ -n "$FW_CRC" && "$FW_CRC" == "$HOST_CRC" ]]; then
    echo "PASS"
else
    echo "FAIL"
    exit 1
fi
