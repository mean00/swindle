//! Plain LEDs: the open drain LEDs blink, as in src/swindle_gd32.cpp.
//!
//! Platform-neutral on purpose: the pins come from the probe board table and the
//! line is driven through the esprit `GpioPin` trait, so nothing here names an MCU
//! backend crate.

use crate::registers::BLINK_HALF_PERIOD_MS;
use rs_swindle_board::BOARD;
use rust_esprit::hal::GpioMode;
use rust_esprit::{HalGpioPin, Pin, delay_ms};

pub fn run() -> ! {
    /* The GPIO port clocks are already on: esprit's C++ main() resets and enables
     * GPIOA / GPIOB / GPIOC (and AF) before any Rust code runs
     * (esprit/mcus/common_bluepill/lnSystemInit.cpp). */
    for led in BOARD.leds {
        let mut pin = Pin::from(*led as u32);
        HalGpioPin::set_mode(&mut pin, GpioMode::OutputOpenDrain);
    }
    let mut on = true;
    loop {
        delay_ms(BLINK_HALF_PERIOD_MS);
        for led in BOARD.leds {
            let mut pin = Pin::from(*led as u32);
            HalGpioPin::open_drain_close(&mut pin, on);
        }
        on = !on;
    }
}
