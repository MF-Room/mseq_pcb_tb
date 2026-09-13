use cortex_m::peripheral::DWT;
use crc::{CRC_32_ISO_HDLC, Crc};
use embedded_hal_nb::serial::Read;
use log::{debug, info, trace};
use stm32f4xx_hal::{
    pac,
    prelude::*,
    rcc::Rcc,
    serial::{Config, Serial, config::StopBits},
};

const WATCHDOG_CYCLES: u32 = crate::SYSCLK_MHZ * 1_000 * 2_000;
static CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

/// MIDI IN 1: USART1 RX on PB3.
#[cfg(not(receive2_mode))]
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

    let (_, rx) = serial.split();

    debug!("Receive mode (MIDI IN 1)");
    receive(rx)
}

/// MIDI IN 2: USART2 RX on PA3. There is no TX pin on this port.
#[cfg(receive2_mode)]
pub fn run_in2(usart2: pac::USART2, gpioa: pac::GPIOA, rcc: &mut Rcc) -> ! {
    let gpioa = gpioa.split(rcc);
    let rx_pin = gpioa.pa3.into_alternate::<7>();

    let rx = Serial::rx(
        usart2,
        rx_pin,
        Config::default()
            .baudrate(31_250.bps())
            .wordlength_8()
            .parity_none()
            .stopbits(StopBits::STOP1),
        rcc,
    )
    .unwrap();

    debug!("Receive mode (MIDI IN 2)");
    receive(rx)
}

/// Computes a CRC32 over the received channel message bytes and logs it once
/// nothing has arrived for the watchdog period.
fn receive(mut rx: impl Read<u8>) -> ! {
    let mut digest = CRC32.digest();
    let mut byte_count: u32 = 0;
    let mut last_rx: Option<u32> = None;
    let mut sysex_active = false;

    loop {
        match rx.read() {
            Ok(byte) => {
                if byte >= 0xF8 {
                    // System Real-Time: ignore
                } else if byte == 0xF0 {
                    sysex_active = true;
                } else if byte == 0xF7 {
                    sysex_active = false;
                } else if byte >= 0xF1 {
                    // Other System Common: ignore
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
                if let Some(t) = last_rx
                    && DWT::cycle_count().wrapping_sub(t) >= WATCHDOG_CYCLES
                {
                    info!("{:#010X} ({} bytes)", digest.finalize(), byte_count);
                    cortex_m::asm::bkpt();
                    loop {
                        cortex_m::asm::wfi();
                    }
                }
            }
            Err(nb::Error::Other(_)) => {}
        }
    }
}
