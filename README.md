# MSeq PCB Testbench

[![CI](https://github.com/MF-Room/mseq_pcb_tb/actions/workflows/ci.yml/badge.svg)](https://github.com/MF-Room/mseq_pcb_tb/actions/workflows/ci.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

Automated tests for the MSeq PCB (STM32F413CHU6).

The `firmware` runs on the target and `midi-tester` drives a USB-MIDI adapter from the host machine.

## Requirements

- STLink connected to the target
- USB-MIDI adapter connected to the host
- Rust with the `thumbv7em-none-eabihf` target, `probe-rs` and `flip-link`
- For the MIDI tests: `./setup_midi_ports.sh` run once. It lists the host MIDI ports and asks which one is wired to each of the board's MIDI IN 1, IN 2, OUT and THRU, the same port possibly several times, and writes them to `midi_ports.conf` for the receive, send and THRU tests
- GNU `timeout`
- For the USB flashing test: USB-C cable to the board, the `llvm-tools` rustup component (for `rust-objcopy`) and `stm32flash`

## Tests

**Receive test** (RX path): the host sends 3000 MIDI messages into the input under test, the MCU receives them and computes a CRC32. Both CRCs are compared. The argument selects the input under test: `1` for MIDI IN 1 (USART1, default), `2` for MIDI IN 2 (USART2).

```
./test_receive.sh      # MIDI IN 1
./test_receive.sh 2    # MIDI IN 2
```

**Send test** (TX path): the MCU sends 3000 MIDI messages, the host receives them from MIDI OUT and computes a CRC32. Both CRCs are compared.

```
./test_send.sh
```

**Display test**: the MCU generates a random number and shows it on the LCD. The user reads it and types it in the terminal. PASS if it matches.

```
./test_display.sh
```

**BPM test**: the MCU generates 50 beats at 120 BPM using the LSE/RTC. The host measures elapsed wall-clock time and checks the result is within 0.5 BPM of the target.

```
./test_bpm.sh
```

Both SPI memory tests run their full sequence at every SPI2 speed the MCU can generate, 3.125, 6.25, 12.5 and 25 MHz (APB1 50 MHz divided by 16, 8, 4 and 2; 25 MHz is the maximum), with a different data pattern each time. The log ends with one OK/FAIL line per speed, and the test passes only if every speed passes.

**NOR flash test** (W25Q16JV on SPI2): the MCU checks the JEDEC ID, erases the last 4 KB sector, checks it is blank, programs a pseudo-random pattern and reads it back. Destructive: the sector content is lost. No USB-MIDI adapter needed.

```
./test_nor.sh
```

**FRAM test** (MB85RS256B on SPI2): the MCU checks the device ID (`04 7F 05 09`) and that the write enable latch toggles, writes a pseudo-random pattern over the whole 32 KB and reads it back with both READ (rated up to 25 MHz) and FSTRD (fast read, rated up to 33 MHz), checks every address line for aliasing, and checks a write without write enable is ignored. Destructive: the whole memory is overwritten. No USB-MIDI adapter needed.

```
./test_fram.sh
```

**THRU test**: the host sends 3000 MIDI messages into MIDI IN 1, the PCB copies them in hardware to MIDI THRU, and the host receives them from there. Both CRCs are compared. The receive firmware is flashed first so the MCU pin on the IN 1 line is an input.

```
./test_thru.sh
```

**Master/slave switch test** (SW3): the MCU logs the switch level, the user flips the switch and flips it back within 30 s each, and the MCU checks both changes.

```
./test_switch.sh
```

**USB flashing test** (CP2102N on USART1, BOOT and RESET buttons): the user enters the STM32 bootloader (hold BOOT, press RESET, release BOOT), then `stm32flash` writes and verifies the receive firmware over USB without the STLink. PASS if the STM32F413 is detected and the verify succeeds. The board is `/dev/ttyUSB0` on Linux and the first `/dev/cu.usbserial*` on macOS; set `SERIAL=<device>` to override.

```
./test_usb_flash.sh
```
