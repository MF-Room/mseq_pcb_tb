//! OLED display test. The display is an HS242L03B2C01: a 2.42" 128x64 module with an SSD1309
//! controller, its own panel boost converter and I2C pull-ups, on header J1 (I2C1: PB6 SCL,
//! PB7 SDA). The firmware shows a random number in large digits and logs it; the user reads it
//! back.

use cortex_m::peripheral::DWT;
use log::{error, info};
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use stm32f4xx_hal::{i2c::I2c, pac, prelude::*, rcc::Rcc};

const WIDTH: usize = 128;
const HEIGHT: usize = 64;
const PAGES: usize = HEIGHT / 8;

/// SSD1309 7-bit I2C addresses, selected by its SA0 pin on the module
const ADDRESSES: [u8; 2] = [0x3C, 0x3D];
/// Control bytes: the rest of the transfer is a command stream, or display data
const CMD: u8 = 0x00;
const DATA: u8 = 0x40;
const NOP: u8 = 0xE3;
const DISPLAY_ON: u8 = 0xAF;

/// Initialisation from the module datasheet (section 4.2): command unlock, display off,
/// clock A0h, 1/64 multiplex, no offset, start line 0, column 127 on SEG0 and remapped COM scan
/// (the vendor's upright orientation), alternative COM pins, contrast 7Fh, pre-charge 82h,
/// VCOMH 34h, display follows RAM, not inverted. The panel supply comes from the module's boost
/// converter, so unlike an SSD1306 there is no charge pump to enable.
const INIT: &[u8] = &[
    0xFD, 0x12, 0xAE, 0xD5, 0xA0, 0xA8, 0x3F, 0xD3, 0x00, 0x40, 0xA1, 0xC8, 0xDA, 0x12, 0x81, 0x7F,
    0xD9, 0x82, 0xDB, 0x34, 0xA4, 0xA6,
];

/// 5x7 digits, one byte per column, bit 0 at the top
const DIGITS: [[u8; 5]; 10] = [
    [0x3E, 0x51, 0x49, 0x45, 0x3E],
    [0x00, 0x42, 0x7F, 0x40, 0x00],
    [0x42, 0x61, 0x51, 0x49, 0x46],
    [0x21, 0x41, 0x45, 0x4B, 0x31],
    [0x18, 0x14, 0x12, 0x7F, 0x10],
    [0x27, 0x45, 0x45, 0x45, 0x39],
    [0x3C, 0x4A, 0x49, 0x49, 0x30],
    [0x01, 0x71, 0x09, 0x05, 0x03],
    [0x36, 0x49, 0x49, 0x49, 0x36],
    [0x06, 0x49, 0x49, 0x29, 0x1E],
];
/// Digit magnification: 4-digit numbers are 92 x 28 pixels
const SCALE: usize = 4;

/// One bit per pixel, laid out like the SSD1309 RAM: 8 pages of 8 rows, bit 0 at the top
type Frame = [[u8; WIDTH]; PAGES];

pub fn run(i2c1: pac::I2C1, gpiob: pac::GPIOB, rcc: &mut Rcc) -> ! {
    // Lets the module's boost converter and power-on reset settle when the board was just powered
    cortex_m::asm::delay(crate::SYSCLK_MHZ * 1_000 * 100);

    // The module's pull-ups keep both lines high. The HAL waits forever for a START it cannot
    // generate, so check the lines first, against the internal pull-downs
    let gpiob = gpiob.split(rcc);
    let scl = gpiob.pb6.into_pull_down_input();
    let sda = gpiob.pb7.into_pull_down_input();
    cortex_m::asm::delay(crate::SYSCLK_MHZ * 10);
    let (scl_high, sda_high) = (scl.is_high(), sda.is_high());
    if !(scl_high && sda_high) {
        error!(
            "I2C lines not idle (SCL {}, SDA {}): display not connected or not powered",
            level(scl_high),
            level(sda_high)
        );
        stop();
    }

    let scl = scl.into_alternate::<4>().set_open_drain();
    let sda = sda.into_alternate::<4>().set_open_drain();
    let mut i2c = I2c::new(i2c1, (scl, sda), 100.kHz(), rcc);

    let Some(addr) = ADDRESSES
        .into_iter()
        .find(|&a| i2c.write(a, &[CMD, NOP]).is_ok())
    else {
        error!(
            "No display answered at 0x3C or 0x3D: check SCL and SDA are not swapped \
             (module pins GND VCC SCL SDA, J1 pins SCL SDA 3V3 GND)"
        );
        stop();
    };
    info!("SSD1309 at {:#04X}", addr);

    let mut rng = SmallRng::seed_from_u64(DWT::cycle_count() as u64);
    let number: u16 = rng.random_range(0u16..=9999);

    let mut frame: Frame = [[0; WIDTH]; PAGES];
    draw_number(&mut frame, number);

    // RAM is written while the display is off, so no stale content from a previous run shows
    let shown = send(&mut i2c, addr, CMD, INIT.iter().copied())
        .and_then(|_| write_frame(&mut i2c, addr, &frame))
        .and_then(|_| send(&mut i2c, addr, CMD, [DISPLAY_ON]));
    if let Err(e) = shown {
        error!("I2C error: {:?}", e);
        stop();
    }

    info!("Number: {}", number);
    stop();
}

/// Sends `bytes` in one transfer after the control byte
fn send(
    i2c: &mut I2c<pac::I2C1>,
    addr: u8,
    control: u8,
    bytes: impl IntoIterator<Item = u8>,
) -> Result<(), stm32f4xx_hal::i2c::Error> {
    i2c.write_iter(addr, core::iter::once(control).chain(bytes))
}

/// Writes the whole RAM, one page at a time in page addressing mode (the reset mode)
fn write_frame(
    i2c: &mut I2c<pac::I2C1>,
    addr: u8,
    frame: &Frame,
) -> Result<(), stm32f4xx_hal::i2c::Error> {
    for (page, columns) in (0u8..).zip(frame) {
        // Page start address, then column 0 (lower and upper nibble)
        send(i2c, addr, CMD, [0xB0 | page, 0x00, 0x10])?;
        send(i2c, addr, DATA, columns.iter().copied())?;
    }
    Ok(())
}

/// Draws `number` centred, in 5x7 digits magnified SCALE times
fn draw_number(frame: &mut Frame, number: u16) {
    let mut digits = [0u8; 5];
    let mut len = 0;
    let mut n = number;
    loop {
        digits[len] = (n % 10) as u8;
        len += 1;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    digits[..len].reverse();

    // Each digit takes 5 columns plus a 1-column gap, except the last one
    let x0 = (WIDTH - (len * 6 - 1) * SCALE) / 2;
    let y0 = (HEIGHT - 7 * SCALE) / 2;
    for (i, &d) in digits[..len].iter().enumerate() {
        for (col, bits) in DIGITS[usize::from(d)].iter().enumerate() {
            for row in (0..7).filter(|row| bits & (1 << row) != 0) {
                let x = x0 + (i * 6 + col) * SCALE;
                let y = y0 + row * SCALE;
                for yy in y..y + SCALE {
                    for column in &mut frame[yy / 8][x..x + SCALE] {
                        *column |= 1 << (yy % 8);
                    }
                }
            }
        }
    }
}

fn level(high: bool) -> &'static str {
    if high { "high" } else { "low" }
}

/// Ends the test: probe-rs exits on the breakpoint
fn stop() -> ! {
    cortex_m::asm::bkpt();
    loop {
        cortex_m::asm::wfi();
    }
}
