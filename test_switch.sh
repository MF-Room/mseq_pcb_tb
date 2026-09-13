#!/usr/bin/env bash
# Tests the MASTER/SLAVE switch (SW3 on PA1): firmware logs the level, the user flips the
# switch and back, and firmware checks both changes. Prints PASS if the firmware reports "SWITCH PASS".
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FW_OUT=$(mktemp /tmp/fw_out.XXXXXX)

cleanup() {
    rm -f "$FW_OUT"
}
trap cleanup EXIT

echo "=== Building firmware (switch mode) ==="
cd "$REPO/firmware"
MODE=switch cargo build --release

echo "=== Flashing firmware ==="
echo "When asked, flip the MASTER/SLAVE switch, then flip it back (30 s per flip)."
echo ""
# Output is shown live so the prompts are visible; the timeout covers a panic
timeout 90 probe-rs run --chip STM32F413CHUx \
    target/thumbv7em-none-eabihf/release/firmware \
    2>/dev/null | tee "$FW_OUT" || true

echo ""
if grep -q "SWITCH PASS" "$FW_OUT"; then
    echo "PASS"
else
    echo "FAIL"
    exit 1
fi
