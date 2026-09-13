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

    let spi = Spi::new(
        spi2,
        (Some(sck), Some(miso), Some(mosi)),
        MODE_0,
        4.MHz(),
        rcc,
    );

    (spi, cs_nor, cs_fram)
}

/// Index of the first byte that differs between `actual` and `expected`.
pub fn first_mismatch(actual: &[u8], expected: &[u8]) -> Option<usize> {
    actual.iter().zip(expected).position(|(a, e)| a != e)
}

/// Logs the final verdict and stops, which makes `probe-rs run` exit.
pub fn finish(name: &str, result: Result<(), ()>) -> ! {
    match result {
        Ok(()) => info!("{} PASS", name),
        Err(()) => error!("{} FAIL", name),
    }
    cortex_m::asm::bkpt();
    loop {
        cortex_m::asm::wfi();
    }
}
