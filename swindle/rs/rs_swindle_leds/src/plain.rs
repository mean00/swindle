//! Plain LEDs: the open drain LEDs blink, as in src/swindle_gd32.cpp.

use crate::registers::*;
use rs_gpio_bluepill::{Mode, Pin, open_drain_close, set_mode};
use rs_rcu_bluepill::{Peripheral, enable};
use rust_esprit::delay_ms;

/// Number of pins per GPIO port (port = pin / PINS_PER_PORT)
const PINS_PER_PORT: u32 = 16;

fn port_clock(pin: Pin) -> Peripheral {
    match (pin as u32) / PINS_PER_PORT {
        0 => Peripheral::GpioA,
        1 => Peripheral::GpioB,
        _ => Peripheral::GpioC,
    }
}

pub fn run() -> ! {
    for pin in LED_PINS {
        enable(port_clock(pin));
        set_mode(pin, Mode::OutputOpenDrain, GPIO_DEFAULT_SPEED_MHZ);
    }
    let mut on = true;
    loop {
        delay_ms(BLINK_HALF_PERIOD_MS);
        for pin in LED_PINS {
            open_drain_close(pin, on);
        }
        on = !on;
    }
}
