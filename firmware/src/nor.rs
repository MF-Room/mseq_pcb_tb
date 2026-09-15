use crate::spi_bus::{self, Device};
use cortex_m::peripheral::DWT;
use log::{error, info};
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use stm32f4xx_hal::{pac, rcc::Rcc};

// W25Q16JV commands (datasheet revision F)
const WRITE_ENABLE: u8 = 0x06;
const READ_STATUS_1: u8 = 0x05;
const WRITE_STATUS_1: u8 = 0x01;
const READ_STATUS_2: u8 = 0x35;
const WRITE_STATUS_2: u8 = 0x31;
const READ_STATUS_3: u8 = 0x15;
const WRITE_STATUS_3: u8 = 0x11;
const READ_DATA: u8 = 0x03;
const PAGE_PROGRAM: u8 = 0x02;
const SECTOR_ERASE: u8 = 0x20;
const JEDEC_ID: u8 = 0x9F;
const RELEASE_POWER_DOWN: u8 = 0xAB;

const STATUS_1_BUSY: u8 = 0x01;
const STATUS_1_WEL: u8 = 0x02;
/// BP0-2, TB and SEC block protection bits.
const STATUS_1_PROTECT: u8 = 0x7C;
/// Status register lock: status registers cannot be written.
const STATUS_2_SRL: u8 = 0x01;
/// Complement protect: inverts the SR1 block protection.
const STATUS_2_CMP: u8 = 0x40;
/// Write protect selection: use the individual block locks, all locked at power-up.
const STATUS_3_WPS: u8 = 0x04;

/// Last 4 KB sector, so the top address bits are exercised.
const SECTOR_ADDR: u32 = 0x1F_F000;
const SECTOR_LEN: usize = 4096;
const PAGE_LEN: usize = 256;

// Datasheet maximums are 400 ms for a sector erase, 3 ms for a page program
// and 15 ms for a status register write.
const ERASE_TIMEOUT_MS: u32 = 1_000;
const PROGRAM_TIMEOUT_MS: u32 = 20;
const STATUS_WRITE_TIMEOUT_MS: u32 = 20;

const SEED: u64 = 42;

pub fn run(spi2: pac::SPI2, gpiob: pac::GPIOB, rcc: &mut Rcc) -> ! {
    let (spi, cs_nor, _cs_fram) = spi_bus::init(spi2, gpiob, rcc);
    let nor = Device::new(spi, cs_nor);
    info!("NOR test: W25Q16JV, sector {:#08X}", SECTOR_ADDR);
    spi_bus::run_speeds("NOR", nor, rcc, test);
}

/// Full check at the current SPI speed. The pattern seed changes with `run` so data
/// left by a previous speed cannot pass the verify.
fn test(nor: &mut Device, run: u64) -> Result<(), ()> {
    nor.write(&[RELEASE_POWER_DOWN], &[]);
    // tRES1 is 3 us, wait 1 ms
    let start = DWT::cycle_count();
    while elapsed_ms(start) < 1 {}

    check_id(nor)?;
    clear_protection(nor)?;

    let mut buf = [0u8; SECTOR_LEN];

    write_enable(nor)?;
    nor.write(&addr_cmd(SECTOR_ERASE, SECTOR_ADDR), &[]);
    wait_ready(nor, ERASE_TIMEOUT_MS)?;
    nor.read(&addr_cmd(READ_DATA, SECTOR_ADDR), &mut buf);
    if let Some(i) = buf.iter().position(|&b| b != 0xFF) {
        error!(
            "Erase: {:#08X} reads {:#04X}, expected 0xFF",
            SECTOR_ADDR as usize + i,
            buf[i]
        );
        return Err(());
    }
    info!("Erase OK");

    let mut rng = SmallRng::seed_from_u64(SEED + run);
    let mut pattern = [0u8; SECTOR_LEN];
    pattern.iter_mut().for_each(|b| *b = rng.random());

    for (i, page) in pattern.chunks(PAGE_LEN).enumerate() {
        let addr = SECTOR_ADDR + (i * PAGE_LEN) as u32;
        write_enable(nor)?;
        nor.write(&addr_cmd(PAGE_PROGRAM, addr), page);
        wait_ready(nor, PROGRAM_TIMEOUT_MS)?;
    }
    info!("Program OK");

    nor.read(&addr_cmd(READ_DATA, SECTOR_ADDR), &mut buf);
    if let Some(i) = spi_bus::first_mismatch(&buf, &pattern) {
        error!(
            "Verify: {:#08X} reads {:#04X}, expected {:#04X}",
            SECTOR_ADDR as usize + i,
            buf[i],
            pattern[i]
        );
        return Err(());
    }
    info!("Verify OK ({} bytes)", SECTOR_LEN);

    Ok(())
}

fn check_id(nor: &mut Device) -> Result<(), ()> {
    let mut id = [0u8; 3];
    nor.read(&[JEDEC_ID], &mut id);
    info!("JEDEC ID: {:02X} {:02X} {:02X}", id[0], id[1], id[2]);
    // Manufacturer Winbond, type 0x40 (IQ) or 0x70 (IM), capacity 16 Mbit
    if id[0] != 0xEF || !matches!(id[1], 0x40 | 0x70) || id[2] != 0x15 {
        error!("Unexpected JEDEC ID, expected EF 40 15 or EF 70 15");
        return Err(());
    }
    Ok(())
}

/// Clears every setting that makes the chip silently ignore erase and program:
/// SR1 block protection, CMP (which inverts it) and WPS (which switches to per-block locks).
fn clear_protection(nor: &mut Device) -> Result<(), ()> {
    let sr1 = read_register(nor, READ_STATUS_1);
    let sr2 = read_register(nor, READ_STATUS_2);
    let sr3 = read_register(nor, READ_STATUS_3);
    info!("Status registers: {:#04X} {:#04X} {:#04X}", sr1, sr2, sr3);

    let protected =
        sr1 & STATUS_1_PROTECT != 0 || sr2 & STATUS_2_CMP != 0 || sr3 & STATUS_3_WPS != 0;
    if !protected {
        return Ok(());
    }
    if sr2 & STATUS_2_SRL != 0 {
        error!("Write protection is set but the status registers are locked (SRL)");
        return Err(());
    }

    clear_bits(
        nor,
        "block protection",
        READ_STATUS_1,
        WRITE_STATUS_1,
        STATUS_1_PROTECT,
    )?;
    clear_bits(nor, "CMP", READ_STATUS_2, WRITE_STATUS_2, STATUS_2_CMP)?;
    clear_bits(nor, "WPS", READ_STATUS_3, WRITE_STATUS_3, STATUS_3_WPS)
}

/// Clears `mask` in one status register, keeping its other bits.
fn clear_bits(
    nor: &mut Device,
    name: &str,
    read_cmd: u8,
    write_cmd: u8,
    mask: u8,
) -> Result<(), ()> {
    let value = read_register(nor, read_cmd);
    if value & mask == 0 {
        return Ok(());
    }
    info!("Clearing {}", name);
    write_enable(nor)?;
    nor.write(&[write_cmd, value & !mask], &[]);
    wait_ready(nor, STATUS_WRITE_TIMEOUT_MS)?;
    let value = read_register(nor, read_cmd);
    if value & mask != 0 {
        error!("{} still set: {:#04X}", name, value);
        return Err(());
    }
    Ok(())
}

fn write_enable(nor: &mut Device) -> Result<(), ()> {
    nor.write(&[WRITE_ENABLE], &[]);
    let status = read_register(nor, READ_STATUS_1);
    if status & STATUS_1_WEL == 0 {
        error!("Write enable latch not set: {:#04X}", status);
        return Err(());
    }
    Ok(())
}

fn read_register(nor: &mut Device, cmd: u8) -> u8 {
    let mut value = [0u8];
    nor.read(&[cmd], &mut value);
    value[0]
}

fn wait_ready(nor: &mut Device, timeout_ms: u32) -> Result<(), ()> {
    let start = DWT::cycle_count();
    while read_register(nor, READ_STATUS_1) & STATUS_1_BUSY != 0 {
        if elapsed_ms(start) > timeout_ms {
            error!("Still busy after {} ms", timeout_ms);
            return Err(());
        }
    }
    Ok(())
}

/// Milliseconds since `start`, from the DWT cycle counter.
fn elapsed_ms(start: u32) -> u32 {
    DWT::cycle_count().wrapping_sub(start) / (crate::SYSCLK_MHZ * 1_000)
}

fn addr_cmd(cmd: u8, addr: u32) -> [u8; 4] {
    let [_, a2, a1, a0] = addr.to_be_bytes();
    [cmd, a2, a1, a0]
}
