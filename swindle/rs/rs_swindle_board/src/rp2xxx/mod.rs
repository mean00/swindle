//! RP2040 / RP2350 probes: the wiring of the RP zero and RP carrier boards, and
//! the code that drives the board's own signals.
//!
//! The pin tables (`rp_zero.rs` / `rp_carrier.rs`, selected in `lib.rs`) are plain
//! data like the LN ones, but nothing renders them as a C header yet: those
//! platforms still take their pinout from `swindle/src/platform/rp2040/
//! bmp_pinout_rp2040*.h` (see `swindle/board.cmake`). The nRST line already
//! comes from here.

pub(crate) mod pins;
pub mod reset_pin;
