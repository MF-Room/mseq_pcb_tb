#!/usr/bin/env bash
# Tests flashing over USB without a debug probe: the STM32 ROM bootloader is entered with the
# BOOT and RESET buttons, and stm32flash writes and verifies the firmware through the CP2102N (USART1).
# Prints PASS if the STM32F413 is detected and the write verifies.
# Usage: SERIAL=/dev/ttyUSB0 ./test_usb_flash.sh
set -euo pipefail
SERIAL=${SERIAL:-/dev/ttyUSB0}
F413_ID="0x0463"

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN=$(mktemp /tmp/fw_bin.XXXXXX)
FLASH_OUT=$(mktemp /tmp/flash_out.XXXXXX)

cleanup() {
    rm -f "$BIN" "$FLASH_OUT"
}
trap cleanup EXIT

echo "=== Building firmware (receive mode) ==="
cd "$REPO/firmware"
cargo build --release
# rust-objcopy ships with the Rust toolchain (llvm-tools)
OBJCOPY="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/rust-objcopy"
"$OBJCOPY" -O binary target/thumbv7em-none-eabihf/release/firmware "$BIN"

echo ""
echo "Connect the USB-C cable, then enter the bootloader:"
echo "  1. hold BOOT"
echo "  2. press and release RESET"
echo "  3. release BOOT"
read -rp "Press Enter when done: "

echo ""
echo "=== Flashing over $SERIAL ==="
# -w write, -v verify, -g start execution at the flash base
STATUS=0
stm32flash -w "$BIN" -v -g 0x08000000 "$SERIAL" 2>&1 | tee "$FLASH_OUT" || STATUS=$?

echo ""
if [[ "$STATUS" -ne 0 ]]; then
    echo "FAIL (stm32flash exited with $STATUS)"
    exit 1
elif ! grep -qi "Device ID *: *$F413_ID" "$FLASH_OUT"; then
    echo "FAIL (STM32F413 device ID $F413_ID not reported)"
    exit 1
else
    echo "PASS"
fi
