use core::fmt::Write;
use cortex_m::peripheral::DWT;
use log::info;
use rand::{RngExt, SeedableRng, rngs::SmallRng};
use stm32f4xx_hal::{pac, prelude::*, rcc::Rcc};

pub fn run(i2c1: pac::I2C1, gpiob: pac::GPIOB, tim3: pac::TIM3, rcc: &mut Rcc) -> ! {
    let gpiob = gpiob.split(rcc);
    let scl = gpiob.pb6.into_alternate::<4>().set_open_drain();
    let sda = gpiob.pb7.into_alternate::<4>().set_open_drain();

    let mut i2c = stm32f4xx_hal::i2c::I2c::new(i2c1, (scl, sda), 50.kHz(), rcc);
    let mut delay = tim3.delay_us(rcc);

    let mut lcd = lcd_lcm1602_i2c::sync_lcd::Lcd::new(&mut i2c, &mut delay)
        .with_address(0x27)
        .with_rows(2)
        .with_cursor_on(false)
        .init()
        .unwrap();

    let mut rng = SmallRng::seed_from_u64(DWT::cycle_count() as u64);
    let number: u16 = rng.random_range(0u16..=9999);

    let mut s = heapless::String::<16>::new();
    write!(s, "{}", number).ok();
    lcd.clear().unwrap();
    lcd.write_str(&s).unwrap();

    info!("Number: {}", number);
    cortex_m::asm::bkpt();
    loop {
        cortex_m::asm::wfi();
    }
}
