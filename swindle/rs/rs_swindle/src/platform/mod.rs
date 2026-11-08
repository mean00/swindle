//! Platform hardware abstraction layer.
//!
//! Provides zero-cost, statically-linked safe interfaces to the hardware
//! drivers in `src/platform/` (SWD bitbang/PIO, clock/timing, ADC voltage, reset pin).

pub mod session;
pub mod swd;
pub mod timing;
pub mod voltage;
pub use crate::reset_pin;
