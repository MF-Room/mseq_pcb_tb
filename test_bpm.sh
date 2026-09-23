#!/usr/bin/env bash
# Tests BPM accuracy using LSE/RTC: firmware generates 50 beats at 100 BPM,
# host measures elapsed wall-clock time between first and last beat RTT message.
set -euo pipefail
command -v timeout >/dev/null || { echo "timeout not found: install GNU coreutils (macOS: brew install coreutils)" >&2; exit 2; }
BEATS=50
TARGET_BPM=120

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FW_PID=""

cleanup() {
    [[ -n "$FW_PID" ]] && kill "$FW_PID" 2>/dev/null || true
    [[ -n "$FW_PID" ]] && wait "$FW_PID" 2>/dev/null || true
}
trap cleanup EXIT

echo "=== Building firmware (bpm mode) ==="
cd "$REPO/firmware"
MODE=bpm cargo build --release

echo "=== Flashing and running (approx. $((BEATS * 60 / TARGET_BPM)) s) ==="
echo ""

T_START_MS=""
T_END_MS=""
BEAT_COUNT=0
PRINT=1

# The MCU stops itself after BEATS beats (about 25 s); the limit covers the RTC never ticking or a panic
while IFS= read -r line; do
    if [[ "$line" == *"BEAT"* ]]; then
        BEAT_COUNT=$((BEAT_COUNT + 1))
        NOW=$(python3 -c "import time; print(int(time.time()*1000))")
        [[ $BEAT_COUNT -eq 1 ]]      && T_START_MS=$NOW
        [[ $BEAT_COUNT -eq $BEATS ]] && { T_END_MS=$NOW; PRINT=0; }
        echo "$line"
    elif [[ $PRINT -eq 1 ]]; then
        echo "$line"
    fi
done < <(MODE=bpm timeout 60 probe-rs run --chip STM32F413CHUx \
    target/thumbv7em-none-eabihf/release/firmware 2>/dev/null)

if [[ $BEAT_COUNT -lt $BEATS ]]; then
    echo ""
    echo "FAIL (only $BEAT_COUNT of $BEATS beats received)"
    exit 1
fi

# (BEATS-1) intervals between beat 1 and beat BEATS
ELAPSED_MS=$(( T_END_MS - T_START_MS ))
EXPECTED_MS=$(( (BEATS - 1) * 60000 / TARGET_BPM ))

MEASURED_BPM=$(python3 -c "print(f'{(${BEATS} - 1) * 60000 / ${ELAPSED_MS}:.2f}')")

echo ""
echo "Expected : ${EXPECTED_MS} ms (${TARGET_BPM} BPM, $((BEATS - 1)) intervals)"
echo "Measured : ${ELAPSED_MS} ms (${MEASURED_BPM} BPM)"

DIFF=$(( ELAPSED_MS - EXPECTED_MS ))
ABS_DIFF=${DIFF#-}
THRESHOLD=$(python3 -c "print(int(${EXPECTED_MS} * 0.5 / ${TARGET_BPM}))")  # 0.5 BPM

if [[ $ABS_DIFF -le $THRESHOLD ]]; then
    echo "PASS"
else
    echo "FAIL"
    exit 1
fi
