use crate::spi_bus::{self, Device};
use crc::{CRC_32_ISO_HDLC, Crc};
use log::{error, info};
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use stm32f4xx_hal::{pac, rcc::Rcc};

// MB85RS256B commands (no RDID on this part)
const WRITE_ENABLE: u8 = 0x06;
const WRITE_DISABLE: u8 = 0x04;
const READ_STATUS: u8 = 0x05;
const WRITE_STATUS: u8 = 0x01;
const READ: u8 = 0x03;
const WRITE: u8 = 0x02;

const STATUS_WEL: u8 = 0x02;
/// BP0 and BP1 block protection bits.
const STATUS_PROTECT: u8 = 0x0C;

const FRAM_LEN: usize = 32 * 1024;
const CHUNK_LEN: usize = 1024;

const SEED: u64 = 42;
static CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

pub fn run(spi2: pac::SPI2, gpiob: pac::GPIOB, rcc: &mut Rcc) -> ! {
    let (spi, _cs_nor, cs_fram) = spi_bus::init(spi2, gpiob, rcc);
    let mut fram = Device::new(spi, cs_fram);
    info!("FRAM test: MB85RS256B, {} bytes", FRAM_LEN);
    spi_bus::finish("FRAM", test(&mut fram));
}

fn test(fram: &mut Device) -> Result<(), ()> {
    // The chip has no ID register, so toggling the write enable latch is the presence check.
    check_write_latch(fram)?;
    clear_protection(fram)?;
    full_pattern(fram)?;
    address_lines(fram)?;
    write_protect(fram)
}

fn check_write_latch(fram: &mut Device) -> Result<(), ()> {
    let status = read_status(fram);
    info!("Status register: {:#04X}", status);

    fram.write(&[WRITE_ENABLE], &[]);
    let enabled = read_status(fram);
    fram.write(&[WRITE_DISABLE], &[]);
    let disabled = read_status(fram);
    if enabled & STATUS_WEL == 0 || disabled & STATUS_WEL != 0 {
        error!(
            "Write enable latch does not toggle: {:#04X} after WREN, {:#04X} after WRDI",
            enabled, disabled
        );
        return Err(());
    }
    info!("Write enable latch OK");
    Ok(())
}

fn clear_protection(fram: &mut Device) -> Result<(), ()> {
    if read_status(fram) & STATUS_PROTECT == 0 {
        return Ok(());
    }
    info!("Clearing block protection");
    fram.write(&[WRITE_ENABLE], &[]);
    fram.write(&[WRITE_STATUS, 0x00], &[]);
    let status = read_status(fram);
    if status & STATUS_PROTECT != 0 {
        error!("Block protection still set: {:#04X}", status);
        return Err(());
    }
    Ok(())
}

/// Writes a pseudo-random pattern over the whole memory, then reads it back.
/// The pattern is regenerated from the seed to avoid holding 32 KB in RAM.
fn full_pattern(fram: &mut Device) -> Result<(), ()> {
    let mut chunk = [0u8; CHUNK_LEN];

    let mut rng = SmallRng::seed_from_u64(SEED);
    let mut digest = CRC32.digest();
    for addr in (0..FRAM_LEN).step_by(CHUNK_LEN) {
        chunk.iter_mut().for_each(|b| *b = rng.random());
        digest.update(&chunk);
        // WEL is cleared after every WRITE
        fram.write(&[WRITE_ENABLE], &[]);
        fram.write(&addr_cmd(WRITE, addr), &chunk);
    }
    info!("Write OK ({:#010X})", digest.finalize());

    let mut rng = SmallRng::seed_from_u64(SEED);
    let mut expected = [0u8; CHUNK_LEN];
    let mut digest = CRC32.digest();
    for addr in (0..FRAM_LEN).step_by(CHUNK_LEN) {
        expected.iter_mut().for_each(|b| *b = rng.random());
        fram.read(&addr_cmd(READ, addr), &mut chunk);
        digest.update(&chunk);
        if let Some(i) = spi_bus::first_mismatch(&chunk, &expected) {
            error!(
                "Verify: {:#06X} reads {:#04X}, expected {:#04X}",
                addr + i,
                chunk[i],
                expected[i]
            );
            return Err(());
        }
    }
    info!("Verify OK ({:#010X})", digest.finalize());
    Ok(())
}

/// Writes a distinct marker at 0x0000 and at every power-of-two address, then reads them
/// all back. A stuck or shorted address line makes two of them alias and one gets overwritten.
fn address_lines(fram: &mut Device) -> Result<(), ()> {
    let addrs = core::iter::once(0).chain((0..15).map(|bit| 1usize << bit));
    let marker = |i: usize| 0xA5 ^ i as u8;

    for (i, addr) in addrs.clone().enumerate() {
        fram.write(&[WRITE_ENABLE], &[]);
        fram.write(&addr_cmd(WRITE, addr), &[marker(i)]);
    }
    for (i, addr) in addrs.enumerate() {
        let mut byte = [0u8];
        fram.read(&addr_cmd(READ, addr), &mut byte);
        if byte[0] != marker(i) {
            error!(
                "Address lines: {:#06X} reads {:#04X}, expected {:#04X}",
                addr,
                byte[0],
                marker(i)
            );
            return Err(());
        }
    }
    info!("Address lines OK");
    Ok(())
}

/// A WRITE without a preceding WREN must be ignored.
fn write_protect(fram: &mut Device) -> Result<(), ()> {
    let mut before = [0u8];
    fram.read(&addr_cmd(READ, 0), &mut before);
    fram.write(&[WRITE_DISABLE], &[]);
    fram.write(&addr_cmd(WRITE, 0), &[!before[0]]);
    let mut after = [0u8];
    fram.read(&addr_cmd(READ, 0), &mut after);
    if after != before {
        error!(
            "Write without WREN changed 0x0000 from {:#04X} to {:#04X}",
            before[0], after[0]
        );
        return Err(());
    }
    info!("Write protection OK");
    Ok(())
}

fn read_status(fram: &mut Device) -> u8 {
    let mut status = [0u8];
    fram.read(&[READ_STATUS], &mut status);
    status[0]
}

fn addr_cmd(cmd: u8, addr: usize) -> [u8; 3] {
    let [_, _, a1, a0] = (addr as u32).to_be_bytes();
    [cmd, a1, a0]
}
