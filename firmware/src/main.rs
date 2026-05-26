#![no_std]
#![no_main]

mod rtt_logger;

use cortex_m::peripheral::DWT;
use crc::{Crc, CRC_32_ISO_HDLC};
use embedded_hal_nb::serial::Read;
use log::{info, trace};
use panic_rtt_target as _;
use stm32f4xx_hal::{
    pac,
    prelude::*,
    serial::{Config, Serial, config::StopBits},
};

const SYSCLK_MHZ: u32 = 84;
const WATCHDOG_CYCLES: u32 = SYSCLK_MHZ * 1_000 * 2_000;

static CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);
static mut LOGGER: rtt_logger::RttLogger = rtt_logger::RttLogger {
    level: log::LevelFilter::Off,
};

#[cortex_m_rt::entry]
fn main() -> ! {
    unsafe { (*core::ptr::addr_of_mut!(LOGGER)).init(log::LevelFilter::Trace) };
    info!("MIDI testbench ready — waiting for bytes...");

    let dp = pac::Peripherals::take().unwrap();
    let mut cp = cortex_m::Peripherals::take().unwrap();

    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.use_hse(25.MHz()).sysclk(SYSCLK_MHZ.MHz()).freeze();

    cp.DCB.enable_trace();
    cp.DWT.enable_cycle_counter();

    // USART1: PA15 (TX, AF7) / PB3 (RX, AF7) at 31250 baud (standard MIDI)
    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();
    let tx_pin = gpioa.pa15.into_alternate::<7>();
    let rx_pin = gpiob.pb3.into_alternate::<7>();

    let serial = Serial::new(
        dp.USART1,
        (tx_pin, rx_pin),
        Config::default()
            .baudrate(31_250.bps())
            .wordlength_8()
            .parity_none()
            .stopbits(StopBits::STOP1),
        &clocks,
    )
    .unwrap();
    let (_, mut rx) = serial.split();

    let mut digest = CRC32.digest();
    let mut byte_count: u32 = 0;
    let mut last_rx: Option<u32> = None;

    loop {
        match rx.read() {
            Ok(byte) => {
                if byte_count == 0 {
                    info!("Receiving...");
                }
                digest.update(&[byte]);
                byte_count += 1;
                last_rx = Some(DWT::cycle_count());
                trace!("byte {}: {:#04X}", byte_count, byte);
            }
            Err(nb::Error::WouldBlock) => {
                if let Some(t) = last_rx {
                    let elapsed = DWT::cycle_count().wrapping_sub(t);
                    if elapsed >= WATCHDOG_CYCLES {
                        let crc = digest.finalize();
                        info!("Bytes: {} | CRC32: {:#010X}", byte_count, crc);
                        digest = CRC32.digest();
                        byte_count = 0;
                        last_rx = None;
                    }
                }
            }
            Err(nb::Error::Other(_)) => {}
        }
    }
}
