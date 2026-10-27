use core::fmt;
use log::{error, info};
use stm32f4xx_hal::{
    gpio::{AnyPin, Output, PinState, Speed},
    pac,
    prelude::*,
    rcc::Rcc,
    spi::{Mode, Phase, Polarity, Spi},
};

const MODE_0: Mode = Mode {
    polarity: Polarity::IdleLow,
    phase: Phase::CaptureOnFirstTransition,
};

/// SPI2 runs from APB1 (50 MHz) and its prescaler divides by a power of two, at least 2.
/// Each speed is an exact prescaler step, ending at the 25 MHz maximum.
const DIVIDERS: [u32; 4] = [16, 8, 4, 2];

pub type Bus = Spi<pac::SPI2>;
pub type Cs = AnyPin<Output>;

/// One chip on the shared SPI2 bus, selected by its own CS pin.
pub struct Device {
    spi: Bus,
    cs: Cs,
}

impl Device {
    pub fn new(spi: Bus, cs: Cs) -> Self {
        Self { spi, cs }
    }

    /// Re-creates SPI2 with a new clock frequency.
    fn with_speed(self, hz: u32, rcc: &mut Rcc) -> Self {
        let (spi2, pins) = self.spi.release();
        let spi = Spi::new(spi2, pins, MODE_0, hz.Hz(), rcc);
        Self { spi, cs: self.cs }
    }

    /// Sends `cmd` then `data` in a single chip-select frame.
    pub fn write(&mut self, cmd: &[u8], data: &[u8]) {
        self.cs.set_low();
        self.spi.write(cmd).unwrap();
        self.spi.write(data).unwrap();
        self.cs.set_high();
    }

    /// Sends `cmd` then reads `buf.len()` bytes in a single chip-select frame.
    pub fn read(&mut self, cmd: &[u8], buf: &mut [u8]) {
        self.cs.set_low();
        self.spi.write(cmd).unwrap();
        self.spi.read(buf).unwrap();
        self.cs.set_high();
    }
}

/// Sets up SPI2 (PB13 SCK, PB14 MISO, PB15 MOSI) and returns it with the NOR (PB12)
/// and FRAM (PB10) CS pins. Both CS pins start high, so the chip not under test stays off the bus.
pub fn init(spi2: pac::SPI2, gpiob: pac::GPIOB, rcc: &mut Rcc) -> (Bus, Cs, Cs) {
    let gpiob = gpiob.split(rcc);
    let sck = gpiob.pb13.into_alternate::<5>().speed(Speed::High);
    let miso = gpiob.pb14.into_alternate::<5>();
    let mosi = gpiob.pb15.into_alternate::<5>().speed(Speed::High);
    let cs_nor = gpiob
        .pb12
        .into_push_pull_output_in_state(PinState::High)
        .erase();
    let cs_fram = gpiob
        .pb10
        .into_push_pull_output_in_state(PinState::High)
        .erase();

    let hz = rcc.clocks.pclk1().raw() / DIVIDERS[0];
    let spi = Spi::new(
        spi2,
        (Some(sck), Some(miso), Some(mosi)),
        MODE_0,
        hz.Hz(),
        rcc,
    );

    (spi, cs_nor, cs_fram)
}

/// Runs `test` once at every SPI speed, from slowest to fastest, logs a summary and stops,
/// which makes `probe-rs run` exit. `test` gets the speed index, to vary its data pattern.
pub fn run_speeds(
    name: &str,
    mut dev: Device,
    rcc: &mut Rcc,
    test: impl Fn(&mut Device, u64) -> Result<(), ()>,
) -> ! {
    let pclk1 = rcc.clocks.pclk1().raw();
    let mut passed = [false; DIVIDERS.len()];

    for (i, div) in DIVIDERS.iter().enumerate() {
        let hz = pclk1 / div;
        dev = dev.with_speed(hz, rcc);
        info!("--- {} at {} ---", name, Mhz(hz));
        passed[i] = test(&mut dev, i as u64).is_ok();
    }

    for (div, ok) in DIVIDERS.iter().zip(passed) {
        let hz = pclk1 / div;
        if ok {
            info!("{} at {}: OK", name, Mhz(hz));
        } else {
            error!("{} at {}: FAIL", name, Mhz(hz));
        }
    }
    if passed.iter().all(|&ok| ok) {
        info!("{} PASS", name);
    } else {
        error!("{} FAIL", name);
    }

    cortex_m::asm::bkpt();
    loop {
        cortex_m::asm::wfi();
    }
}

/// Formats a frequency in Hz as MHz with three decimals, e.g. "3.125 MHz".
struct Mhz(u32);

impl fmt::Display for Mhz {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}.{:03} MHz",
            self.0 / 1_000_000,
            self.0 % 1_000_000 / 1_000
        )
    }
}

/// Index of the first byte that differs between `actual` and `expected`.
pub fn first_mismatch(actual: &[u8], expected: &[u8]) -> Option<usize> {
    actual.iter().zip(expected).position(|(a, e)| a != e)
}
