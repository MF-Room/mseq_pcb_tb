use crate::spi_bus::{self, Device};
use cortex_m::peripheral::DWT;
use log::{error, info};
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use stm32f4xx_hal::{pac, rcc::Rcc};

// W25Q16JV commands
const WRITE_ENABLE: u8 = 0x06;
const WRITE_STATUS_1: u8 = 0x01;
const READ_STATUS_1: u8 = 0x05;
const READ_DATA: u8 = 0x03;
const PAGE_PROGRAM: u8 = 0x02;
const SECTOR_ERASE: u8 = 0x20;
const JEDEC_ID: u8 = 0x9F;
const RELEASE_POWER_DOWN: u8 = 0xAB;

const STATUS_BUSY: u8 = 0x01;
const STATUS_WEL: u8 = 0x02;
/// BP0-2, TB and SEC block protection bits.
const STATUS_PROTECT: u8 = 0x7C;

/// Last 4 KB sector, so the top address bits are exercised.
const SECTOR_ADDR: u32 = 0x1F_F000;
const SECTOR_LEN: usize = 4096;
const PAGE_LEN: usize = 256;

// Datasheet maximums are 400 ms for a sector erase and 3 ms for a page program.
const ERASE_TIMEOUT_MS: u32 = 1_000;
const PROGRAM_TIMEOUT_MS: u32 = 20;

const SEED: u64 = 42;

pub fn run(spi2: pac::SPI2, gpiob: pac::GPIOB, rcc: &mut Rcc) -> ! {
    let (spi, cs_nor, _cs_fram) = spi_bus::init(spi2, gpiob, rcc);
    let mut nor = Device::new(spi, cs_nor);
    info!("NOR test: W25Q16JV, sector {:#08X}", SECTOR_ADDR);
    spi_bus::finish("NOR", test(&mut nor));
}

fn test(nor: &mut Device) -> Result<(), ()> {
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

    let mut rng = SmallRng::seed_from_u64(SEED);
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

fn clear_protection(nor: &mut Device) -> Result<(), ()> {
    let status = read_status(nor);
    info!("Status register 1: {:#04X}", status);
    if status & STATUS_PROTECT == 0 {
        return Ok(());
    }
    info!("Clearing block protection");
    write_enable(nor)?;
    nor.write(&[WRITE_STATUS_1, 0x00], &[]);
    wait_ready(nor, PROGRAM_TIMEOUT_MS)?;
    let status = read_status(nor);
    if status & STATUS_PROTECT != 0 {
        error!("Block protection still set: {:#04X}", status);
        return Err(());
    }
    Ok(())
}

fn write_enable(nor: &mut Device) -> Result<(), ()> {
    nor.write(&[WRITE_ENABLE], &[]);
    let status = read_status(nor);
    if status & STATUS_WEL == 0 {
        error!("Write enable latch not set: {:#04X}", status);
        return Err(());
    }
    Ok(())
}

fn read_status(nor: &mut Device) -> u8 {
    let mut status = [0u8];
    nor.read(&[READ_STATUS_1], &mut status);
    status[0]
}

fn wait_ready(nor: &mut Device, timeout_ms: u32) -> Result<(), ()> {
    let start = DWT::cycle_count();
    while read_status(nor) & STATUS_BUSY != 0 {
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
