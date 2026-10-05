#![no_std]
#![allow(non_camel_case_types, non_snake_case)]

//! Background LED activity of Swindle.
//!
//! Rust version of the `loop()` of `src/swindle_gd32.cpp` (plain open drain LEDs)
//! and `src/swindle_gd32_ws2812.cpp` (WS2812B breathing). The variant is chosen by
//! cargo features, set by `swindle/rs/CMakeLists.txt` from the CMake configuration:
//!
//! * `bluepill`           : GD32F3 / CH32V3xx, 3 open drain LEDs blinking
//! * `bluepill` + `ws2812`: GD32F3 / CH32V3xx, one WS2812B breathing (`USE_WS2812`)
//! * no feature (RP2040 / RP2350): empty crate, the LED code is still in C++
//!
//! C++ calls [`swindle_leds_run`] at the end of `loop()`; it never returns.

#[cfg(all(feature = "ws2812", not(feature = "bluepill")))]
compile_error!("Feature `ws2812` is only available for bluepill compatible targets");

#[cfg(feature = "bluepill")]
mod registers;

#[cfg(all(feature = "bluepill", not(feature = "ws2812")))]
mod plain;
#[cfg(all(feature = "bluepill", not(feature = "ws2812")))]
use plain as variant;

#[cfg(feature = "ws2812")]
mod sine_table;
#[cfg(feature = "ws2812")]
mod ws2812;
#[cfg(feature = "ws2812")]
use ws2812 as variant;

/// Run the LED activity for the selected variant. Never returns.
#[cfg(feature = "bluepill")]
pub fn run() -> ! {
    variant::run()
}

/// C API for the C++ side (`loop()` in src/swindle_*.cpp). Never returns.
#[cfg(feature = "bluepill")]
#[unsafe(no_mangle)]
pub extern "C" fn swindle_leds_run() -> ! {
    run()
}
