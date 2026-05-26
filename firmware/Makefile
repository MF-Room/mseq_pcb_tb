CHIP := --chip STM32F413CHUx
GDB ?= arm-none-eabi-gdb
SIZE ?= arm-none-eabi-size
BIN := mseq.bin

flash:
	cargo flash $(CHIP) -- -r

rtt:
	cargo run -r

build:
	cargo build -r

gdb_server:
	$(MAKE) flash_debug
	probe-rs gdb $(CHIP)

gdb:
	 $(GDB) -x init.gdb target/thumbv7em-none-eabihf/debug/firmware

flash_debug:
	cargo flash $(CHIP)

program:
	$(MAKE) build
	cargo objcopy --release -- -O binary $(BIN)
	stm32flash -w $(BIN) -v -g 0x0 /dev/ttyUSB0

size:
	cargo build $(PACKAGE) -r
	$(SIZE) -G target/thumbv7em-none-eabihf/release/kernel

.PHONY: flash rtt build gdb_server gdb flash_debug program size
