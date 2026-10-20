#![no_std]

pub use rsbmp::*;

// Makes sure the C API of the LED crate (swindle_leds_run) ends up in the static library
extern crate rs_swindle_leds;

// Makes sure the pure Rust CH32V00x target driver ends up in the static library
extern crate rs_ch32v00x;
