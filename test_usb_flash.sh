#!/usr/bin/env bash
# Tests flashing over USB without a debug probe: after a power cycle, the STM32 ROM bootloader is
# entered with the BOOT and RESET buttons, and stm32flash writes and verifies the firmware through
# the CP2102N (USART1). Prints PASS if the STM32F413 is detected and the write verifies.
# The serial device defaults to /dev/ttyUSB0 on Linux and the first /dev/cu.usbserial* on macOS.
# Usage: SERIAL=/dev/ttyUSB0 ./test_usb_flash.sh
set -euo pipefail
SERIAL=${SERIAL:-}
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

# The power cycle is required: a preceding probe-rs test leaves the core with halting debug enabled
# and the reset vector catch armed (DHCSR.C_DEBUGEN, DEMCR.VC_CORERESET). Both survive the RESET
# button, which is only a system reset, so BOOT+RESET would halt the core on the bootloader's first
# instruction and stm32flash would time out. Only a power-on reset clears them. USB-C VBUS powers
# the board through SW2, so unplugging the cable or switching SW2 off is a power-on reset.
echo ""
echo "Power-cycle the board, then enter the bootloader:"
echo "  1. unplug the USB-C cable (or switch SW2 off), wait 2 s, plug it back in (switch SW2 on)"
echo "  2. hold BOOT"
echo "  3. press and release RESET"
echo "  4. release BOOT"
read -rp "Press Enter when done: "

# The device only exists once the cable is connected, so it is looked up after the prompt
if [[ -z "$SERIAL" ]]; then
    if [[ "$(uname -s)" == "Darwin" ]]; then
        # CP2102N with the built-in macOS driver (cu.usbserial-*) or the Silicon Labs one
        SERIAL=$(ls /dev/cu.usbserial* /dev/cu.SLAB_USBtoUART* 2>/dev/null | head -n 1 || true)
    else
        SERIAL=/dev/ttyUSB0
    fi
fi
if [[ -z "$SERIAL" || ! -e "$SERIAL" ]]; then
    echo ""
    echo "FAIL (serial device ${SERIAL:-not found}; set SERIAL=<device>)"
    exit 1
fi

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
