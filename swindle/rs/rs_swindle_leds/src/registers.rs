#![allow(dead_code)] // each variant (plain / ws2812) only uses part of the constants
//! Constants of the LED activity (no magic numbers in the logic).
//!
//! The pin *numbers* are not here any more: they are the probe board table's
//! (`rs_swindle_board::BOARD.leds` / `.ws2812`), and the plain variant walks that
//! list through the platform-neutral esprit API. Only the WS2812B pin stays a
//! constant, because the strip driver takes it as a const generic
//! (`Ws2812b<NB_LEDS, PIN>`).

use rust_esprit::Pin;

// --- Plain LEDs (open drain, active low) ---

/// Half period of the blink
pub const BLINK_HALF_PERIOD_MS: u32 = 1000;

// --- WS2812B ---

/// Pin driving the WS2812B (TIM3 CH1 with partial remap, handled by the timer driver).
pub const WS2812_PIN: Pin = Pin::PB4;
/// Number of WS2812B leds
pub const WS2812_NB_LEDS: usize = 1;
/// Index of the led used for the breathing
pub const WS2812_LED_INDEX: usize = 0;
/// Delay between two steps of the breathing
pub const BREATHING_STEP_DELAY_MS: u32 = 6;
/// Increment in the sine table at each step
pub const BREATHING_STEP: usize = 1;
/// Last valid index of the sine table
pub const SINE_LAST_INDEX: usize = 180;

// --- The board table is the source of truth for the pin numbers ---
//
// `WS2812_PIN` above is an enum variant (the GPIO driver's spelling of a pin); the
// number it must have is in the probe board table
// (swindle/rs/rs_swindle_board/src/<board>.rs). The plain LEDs need no such check:
// they are read from `BOARD.leds` directly.
#[cfg(any(feature = "bluepill48", feature = "bluepill64"))]
const _: () = {
    assert!(WS2812_PIN as i32 == match rs_swindle_board::BOARD.ws2812 {
        Some(pin) => pin,
        None => -1,
    });
};
