#![allow(dead_code)] // each variant (plain / ws2812) only uses part of the constants
//! Constants of the LED activity (no magic numbers in the logic).

use rs_gpio_bluepill::Pin;

/// GPIO speed argument meaning "default" (same as lnPinMode in C++).
pub const GPIO_DEFAULT_SPEED_MHZ: u32 = 0;

// --- Plain LEDs (open drain, active low) ---

/// System led (LN_SYSTEM_LED) + the two extra leds of the Swindle boards.
pub const LED_PINS: [Pin; 3] = [Pin::PC13, Pin::PA8, Pin::PB13];
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
