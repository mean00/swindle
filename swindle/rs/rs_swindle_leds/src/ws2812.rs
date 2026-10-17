//! WS2812B LED breathing, as in src/swindle_gd32_ws2812.cpp.

use crate::registers::*;
use crate::sine_table::XSIN;
use rust_esprit::delay_ms;
use ws2812b::Ws2812b;

pub fn run() -> ! {
    // The timer driver configures the pin, the timer and the partial TIM3 remap (PB4)
    let mut strip = Ws2812b::<WS2812_NB_LEDS, WS2812_PIN>::new();
    if strip.begin().is_err() {
        // No LED available, nothing to animate
        loop {
            delay_ms(BREATHING_STEP_DELAY_MS);
        }
    }
    let mut pix: usize = 0;
    let mut rising = true;
    loop {
        strip.set_led_color(WS2812_LED_INDEX, XSIN[pix], 0, 0);
        strip.update();
        if rising {
            pix += BREATHING_STEP;
            if pix > SINE_LAST_INDEX - BREATHING_STEP {
                rising = false;
            }
        } else {
            pix -= BREATHING_STEP;
            if pix == 0 {
                rising = true;
            }
        }
        delay_ms(BREATHING_STEP_DELAY_MS);
    }
}
