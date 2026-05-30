#![no_std]
#![no_main]

#[cfg(bpm_mode)] mod bpm;
#[cfg(display_mode)] mod display;
#[cfg(not(any(bpm_mode, display_mode, send_mode)))] mod receive;
mod rtt_logger;
#[cfg(send_mode)] mod send;

use panic_rtt_target as _;
use rtt_logger::{LOG_LEVEL, RttLogger};
use stm32f4xx_hal::{pac, prelude::*, rcc};

pub const SYSCLK_MHZ: u32 = 84;

static LOGGER: RttLogger = RttLogger::new(LOG_LEVEL);

#[cortex_m_rt::entry]
fn main() -> ! {
    LOGGER.init();

    let dp = pac::Peripherals::take().unwrap();
    let mut cp = cortex_m::Peripherals::take().unwrap();

    let mut rcc = dp
        .RCC
        .freeze(rcc::Config::hse(25.MHz()).sysclk(SYSCLK_MHZ.MHz()));

    cp.DCB.enable_trace();
    cp.DWT.enable_cycle_counter();

    #[cfg(bpm_mode)]
    bpm::run(dp.RTC, dp.PWR, &mut rcc);

    #[cfg(display_mode)]
    display::run(dp.I2C1, dp.GPIOB, dp.TIM3, &mut rcc);

    #[cfg(send_mode)]
    send::run(dp.USART1, dp.GPIOA, dp.GPIOB, &mut rcc);

    #[cfg(not(any(bpm_mode, display_mode, send_mode)))]
    receive::run(dp.USART1, dp.GPIOA, dp.GPIOB, &mut rcc);
}
