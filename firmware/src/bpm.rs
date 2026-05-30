use fugit::ExtU64;
use log::info;
use stm32f4xx_hal::{
    pac,
    rcc::Rcc,
    rtc::{Event, Rtc},
};

const TARGET_BPM: u32 = 120;
const BEATS: u32 = 50;
const PERIOD_US: u64 = 2_500_000 / TARGET_BPM as u64; // 20_833

pub fn run(rtc_periph: pac::RTC, mut pwr: pac::PWR, rcc: &mut Rcc) -> ! {
    let mut rtc = Rtc::new(rtc_periph, rcc, &mut pwr);
    rtc.enable_wakeup(PERIOD_US.micros());

    let mut tick: u32 = 0;
    let mut beat: u32 = 0;

    loop {
        while !rtc.is_pending(Event::Wakeup) {}
        rtc.clear_interrupt(Event::Wakeup);

        tick += 1;
        if tick % 24 == 0 {
            beat += 1;
            info!("BEAT {}", beat);
            if beat >= BEATS {
                break;
            }
        }
    }

    cortex_m::asm::bkpt();
    loop {
        cortex_m::asm::wfi();
    }
}
