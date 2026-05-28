# MSeq PCB Testbench

Automated tests for the MIDI I/O paths on the MSeq PCB (STM32F413CHU6).

The firmware runs on the target and the host crate drives a USB-MIDI adapter. Both sides compute a CRC32 over the transferred bytes and the scripts compare them.

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
