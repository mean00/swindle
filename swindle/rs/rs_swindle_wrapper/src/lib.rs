#![no_std]

pub use rsbmp::*;

// Makes sure the C API of the LED crate (swindle_leds_run) ends up in the static library
extern crate rs_swindle_leds;
