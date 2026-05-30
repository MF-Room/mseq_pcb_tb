use cortex_m::peripheral::DWT;
use crc::{CRC_32_ISO_HDLC, Crc};
use embedded_hal_nb::serial::Write;
use log::{debug, info};
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use stm32f4xx_hal::{
    pac,
    prelude::*,
    rcc::Rcc,
    serial::{Config, Serial, config::StopBits},
};

const INTERVAL_CYCLES: u32 = crate::SYSCLK_MHZ * 1_000 * 5;
const SEED: u64 = 42;
const fn parse_u32(s: &[u8]) -> u32 {
    let mut n: u32 = 0;
    let mut i = 0;
    while i < s.len() {
        n = n * 10 + (s[i] - b'0') as u32;
        i += 1;
    }
    n
}
const MESSAGE_COUNT: u32 = match option_env!("COUNT") {
    Some(s) => parse_u32(s.as_bytes()),
    None => 1000,
};

static CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

pub fn run(usart1: pac::USART1, gpioa: pac::GPIOA, gpiob: pac::GPIOB, rcc: &mut Rcc) -> ! {
    let gpioa = gpioa.split(rcc);
    let gpiob = gpiob.split(rcc);
    let tx_pin = gpioa.pa15.into_alternate::<7>();
    let rx_pin = gpiob.pb3.into_alternate::<7>();

    let serial = Serial::new(
        usart1,
        (tx_pin, rx_pin),
        Config::default()
            .baudrate(31_250.bps())
            .wordlength_8()
            .parity_none()
            .stopbits(StopBits::STOP1),
        rcc,
    )
    .unwrap();

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
    loop {
        cortex_m::asm::wfi();
    }
}
