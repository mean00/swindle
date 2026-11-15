//! Plain LEDs: the open drain LEDs blink, as in src/swindle_gd32.cpp.
//!
//! Platform-neutral on purpose: the pins come from the probe board table and the
//! line is driven through the esprit [`Gpio`] super trait, so nothing here names
//! an MCU backend crate.

use crate::registers::BLINK_HALF_PERIOD_MS;
use rs_swindle_board::BOARD;
use rust_esprit::{Gpio, Pin, PinMode, delay_ms};

pub fn run() -> ! {
    /* The GPIO port clocks are already on: esprit's C++ main() resets and enables
     * GPIOA / GPIOB / GPIOC (and AF) before any Rust code runs
     * (esprit/mcus/common_bluepill/lnSystemInit.cpp). */
    for led in BOARD.leds {
        let mut pin = Pin::from(*led as u32);
        let _ = Gpio::set_mode(&mut pin, PinMode::OutputOpenDrain);
    }
    let mut on = true;
    loop {
        delay_ms(BLINK_HALF_PERIOD_MS);
        for led in BOARD.leds {
            let mut pin = Pin::from(*led as u32);
            let _ = Gpio::open_drain(&mut pin, on);
        }
        on = !on;
    }
}
