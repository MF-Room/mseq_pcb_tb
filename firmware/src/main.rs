#![no_std]
#![no_main]

mod rtt_logger;

use cortex_m::peripheral::DWT;
use crc::{CRC_32_ISO_HDLC, Crc};
#[cfg(not(send_mode))]
use log::trace;
use log::{debug, info};
use panic_rtt_target as _;
use rtt_logger::{LOG_LEVEL, RttLogger};
use stm32f4xx_hal::{
    pac,
    prelude::*,
    serial::{Config, Serial, config::StopBits},
};

const SYSCLK_MHZ: u32 = 84;
#[cfg(not(send_mode))]
const WATCHDOG_CYCLES: u32 = SYSCLK_MHZ * 1_000 * 2_000;
#[cfg(send_mode)]
const INTERVAL_CYCLES: u32 = SYSCLK_MHZ * 1_000 * 5; // 5 ms between messages
#[cfg(send_mode)]
const SEED: u64 = 42;
#[cfg(send_mode)]
const fn parse_u32(s: &[u8]) -> u32 {
    let mut n: u32 = 0;
    let mut i = 0;
    while i < s.len() {
        n = n * 10 + (s[i] - b'0') as u32;
        i += 1;
    }
    n
}
#[cfg(send_mode)]
const MESSAGE_COUNT: u32 = match option_env!("COUNT") {
    Some(s) => parse_u32(s.as_bytes()),
    None => 1000,
};

static CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);
static LOGGER: rtt_logger::RttLogger = RttLogger::new(LOG_LEVEL);

#[cortex_m_rt::entry]
fn main() -> ! {
    LOGGER.init();

    let dp = pac::Peripherals::take().unwrap();
    let mut cp = cortex_m::Peripherals::take().unwrap();

    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.use_hse(25.MHz()).sysclk(SYSCLK_MHZ.MHz()).freeze();

    cp.DCB.enable_trace();
    cp.DWT.enable_cycle_counter();

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

    #[cfg(send_mode)]
    {
        use embedded_hal_nb::serial::Write;
        use rand::{RngExt, SeedableRng, rngs::SmallRng};

        let (mut tx, _) = serial.split();

        debug!("Send mode: {} messages", MESSAGE_COUNT);

        let mut rng = SmallRng::seed_from_u64(SEED);
        let mut digest = CRC32.digest();

        for i in 0..MESSAGE_COUNT {
            let msg = [
                0x90 | rng.random_range(0u8..=15),
                rng.random_range(0u8..=127),
                rng.random_range(1u8..=127),
            ];
            for &b in &msg {
                nb::block!(tx.write(b)).ok();
            }
            digest.update(&msg);
            debug!("Sent {}/{}", i + 1, MESSAGE_COUNT);

            let t = DWT::cycle_count();
            while DWT::cycle_count().wrapping_sub(t) < INTERVAL_CYCLES {}
        }

        info!("{:#010X}", digest.finalize());
        cortex_m::asm::bkpt();
        loop { cortex_m::asm::wfi(); }
    }

    #[cfg(not(send_mode))]
    {
        use embedded_hal_nb::serial::Read;

        let (_, mut rx) = serial.split();

        debug!("Receive mode");

        let mut digest = CRC32.digest();
        let mut byte_count: u32 = 0;
        let mut last_rx: Option<u32> = None;
        let mut sysex_active = false;

        loop {
            match rx.read() {
                Ok(byte) => {
                    if byte >= 0xF8 {
                        // System Real-Time (clock, active sensing, reset): ignore
                    } else if byte == 0xF0 {
                        sysex_active = true;
                    } else if byte == 0xF7 {
                        sysex_active = false;
                    } else if byte >= 0xF1 {
                        // Other System Common (MTC, song position, tune): ignore
                    } else if !sysex_active {
                        if byte_count == 0 {
                            debug!("Receiving");
                        }
                        digest.update(&[byte]);
                        byte_count += 1;
                        last_rx = Some(DWT::cycle_count());
                        trace!("byte {}: {:#04X}", byte_count, byte);
                    }
                }
                Err(nb::Error::WouldBlock) => {
                    if let Some(t) = last_rx {
                        if DWT::cycle_count().wrapping_sub(t) >= WATCHDOG_CYCLES {
                            info!("{:#010X} ({} bytes)", digest.finalize(), byte_count);
                            cortex_m::asm::bkpt();
                            loop { cortex_m::asm::wfi(); }
                        }
                    }
                }
                Err(nb::Error::Other(_)) => {}
            }
        }
    }
}
