//! RP2040 / RP2350 probes: the wiring of the RP board, and the code that drives
//! the board's own signals.
//!
//! The pin table (`rp_carrier.rs`, selected in `lib.rs`) matches
//! `swindle/src/platform/rp2040/bmp_pinout_rp2040.h`. The nRST line comes from here.

pub(crate) mod pins;
pub mod reset_pin;
