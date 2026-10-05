#![no_std]
#![no_main]

// Runs the LED activity of the variant selected by the cargo features
// (`bluepill`, optionally `ws2812`). Panic handler and allocator come from rust_esprit.

#[unsafe(no_mangle)]
pub extern "C" fn main() -> ! {
    rs_swindle_leds::run()
}
