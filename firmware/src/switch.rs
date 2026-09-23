use cortex_m::peripheral::DWT;
use log::{error, info};
use stm32f4xx_hal::{gpio::PinState, pac, prelude::*, rcc::Rcc};

const DEBOUNCE_CYCLES: u32 = crate::SYSCLK_MHZ * 1_000 * 20;
const TIMEOUT_CYCLES: u32 = crate::SYSCLK_MHZ * 1_000 * 30_000;
/// Flip once and back, so both positions are seen changing.
const FLIPS: u32 = 2;

/// MASTER/SLAVE switch SW3 on PA1.
pub fn run(gpioa: pac::GPIOA, rcc: &mut Rcc) -> ! {
    let gpioa = gpioa.split(rcc);
    // SW3 ties PA1 either to GND or to 3.3 V through R5, so no internal pull is needed
    let pin = gpioa.pa1.into_floating_input();
    let read = || {
        if pin.is_high() {
            PinState::High
        } else {
            PinState::Low
        }
    };

    let mut state = read();
    info!("Switch: {}", level(state));

    for flip in 1..=FLIPS {
        info!("Flip the MASTER/SLAVE switch ({}/{})", flip, FLIPS);
        match wait_change(read, state) {
            Some(new) => {
                state = new;
                info!("Switch: {}", level(state));
            }
            None => {
                error!("No change within 30 s");
                finish(false);
            }
        }
    }
    finish(true);
}

/// Waits until the pin has held a level other than `state` for the debounce time.
fn wait_change(read: impl Fn() -> PinState, state: PinState) -> Option<PinState> {
    let start = DWT::cycle_count();
    let mut changed_at = None;
    loop {
        let now = DWT::cycle_count();
        let current = read();
        if current != state {
            let since = *changed_at.get_or_insert(now);
            if now.wrapping_sub(since) >= DEBOUNCE_CYCLES {
                return Some(current);
            }
        } else {
            changed_at = None;
        }
        if now.wrapping_sub(start) >= TIMEOUT_CYCLES {
            return None;
        }
    }
}

fn level(state: PinState) -> &'static str {
    match state {
        PinState::High => "HIGH (3.3 V)",
        PinState::Low => "LOW (GND)",
    }
}

fn finish(ok: bool) -> ! {
    if ok {
        info!("SWITCH PASS");
    } else {
        error!("SWITCH FAIL");
    }
    cortex_m::asm::bkpt();
    loop {
        cortex_m::asm::wfi();
    }
}
