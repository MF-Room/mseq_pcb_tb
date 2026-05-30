# MSeq PCB Testbench

[![CI](https://github.com/MF-Room/mseq_pcb_tb/actions/workflows/ci.yml/badge.svg)](https://github.com/MF-Room/mseq_pcb_tb/actions/workflows/ci.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

Automated tests for the MSeq PCB (STM32F413CHU6).

The `firmware` runs on the target and `midi-tester` drives a USB-MIDI adapter from the host machine.

## Requirements

- STLink connected to the target
- USB-MIDI adapter connected to the host
- `probe-rs`, `cargo`, and `flip-link` installed

## Tests

**Receive test** (RX path): the host sends 3000 MIDI messages on port 0, the MCU receives them and computes a CRC32. Both CRCs are compared.

```
./test_receive.sh
```

**Send test** (TX path): the MCU sends 1000 MIDI messages, the host receives them on port 1 and computes a CRC32. Both CRCs are compared. This test is expected to fail due to a hardware issue on the PCB TX path.

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
