//! The target nRST line of the RP2040 / RP2350 probes — which pin, which
//! polarity, and the GPIO write itself.
//!
//! Same split as the LN half (`../../ln/reset_pin.rs`): the pin is the board
//! table's (`BOARD.dev[DEV_NRST]`) and the access is the esprit `GpioPin` trait,
//! here implemented by `rs_gpio_rp2xxx` straight on SIO / IO_BANK0 / PADS_BANK0.
//! Nothing of the C++ `SwdReset` (`swindle/src/bmp_reset*.cpp`) is on the path any
//! more for these boards.
//!
//! Polarity is the `nrst_inverted` feature, the same switch that used to pick
//! `bmp_reset_inv.cpp` over `bmp_reset.cpp` (`USE_INVERTED_NRST`, a MOSFET on the
//! board):
//!
//!   * `nrst_inverted` off : open drain, assert = pull the line low, release = Hi-Z
//!   * `nrst_inverted` on  : push-pull, assert = drive the line high, release = drive low
//!
//! The policy (`mon enable_reset_pin`, the delays, the C ABI) stays in
//! `rs_swindle::reset_pin`.

use crate::BOARD;
use crate::board_desc::DEV_NRST;
use core::sync::atomic::{AtomicBool, Ordering};
use rust_esprit::gpio_rp2xxx::Pin;
use rust_esprit::hal::GpioMode;
use rust_esprit::HalGpioPin;

/// Pin mode of the line, and how one level maps onto "asserted".
#[cfg(not(feature = "nrst_inverted"))]
mod polarity {
    use super::*;

    /// Open drain: releasing the line leaves it to the external pull-up.
    pub const MODE: GpioMode = GpioMode::OutputOpenDrain;

    /// Closing the drain is what pulls nRST low, i.e. what asserts it.
    pub fn drive(pin: &mut Pin, asserted: bool) {
        HalGpioPin::open_drain_close(pin, asserted);
    }
}

#[cfg(feature = "nrst_inverted")]
mod polarity {
    use super::*;

    /// Push-pull: the MOSFET inverts, so driving the pin high asserts.
    pub const MODE: GpioMode = GpioMode::Output;

    pub fn drive(pin: &mut Pin, asserted: bool) {
        HalGpioPin::write(pin, asserted);
    }
}

/// Last commanded state; the line is not readable through the level shifter.
static ASSERTED: AtomicBool = AtomicBool::new(false);

/// The board table stores pins as plain numbers and `Pin::from` transmutes, so a
/// number past the last pad would be undefined. `GPIO29` is the last one on both
/// the RP2040 and the RP2350.
const _: () = assert!(BOARD.dev[DEV_NRST] >= 0 && BOARD.dev[DEV_NRST] <= Pin::GPIO29 as i32);

/// The target nRST line of the board this firmware was built for.
fn pin() -> Pin {
    Pin::from(BOARD.dev[DEV_NRST] as u32)
}

/// Boot-time setup: released level first, then the pin becomes an output (the
/// latch/SIO state must not be able to glitch the line while the mode changes).
pub fn init() {
    let mut pin = pin();
    polarity::drive(&mut pin, false);
    HalGpioPin::set_mode(&mut pin, polarity::MODE);
    ASSERTED.store(false, Ordering::Relaxed);
}

/// Drive the line to the requested state (no timing, no policy).
pub fn set(asserted: bool) {
    let mut pin = pin();
    polarity::drive(&mut pin, asserted);
    ASSERTED.store(asserted, Ordering::Relaxed);
}

/// Whether the line was last commanded asserted.
pub fn is_asserted() -> bool {
    ASSERTED.load(Ordering::Relaxed)
}
