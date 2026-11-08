//! The target nRST line of the LN probes — which pin, which polarity, and the
//! GPIO write itself.
//!
//! The pin is the board table's (`BOARD.dev[DEV_NRST]`) and the access goes
//! through the esprit `GpioPin` trait, so there is no C++ layer between a reset
//! and the GPIO register any more (it used to be `bmp_reset.cpp` /
//! `bmp_reset_inv.cpp` under `swindle/src`).
//!
//! Polarity is the `nrst_inverted` feature, the same switch that used to pick
//! `bmp_reset_inv.cpp` over `bmp_reset.cpp` (`USE_INVERTED_NRST`, a MOSFET on the
//! board):
//!
//!   * `nrst_inverted` off : open drain, assert = pull the line low, release = Hi-Z
//!   * `nrst_inverted` on  : push-pull, assert = drive the line high, release = drive low
//!
//! Only the hardware half lives here: `mon enable_reset_pin`, the pulse / hold-off
//! delays and the C ABI stay in `rs_swindle::reset_pin`, which decides *when* the
//! line moves. Every call is a single GPIO write, so nothing is inlined.

use crate::BOARD;
use crate::board_desc::DEV_NRST;
use core::sync::atomic::{AtomicBool, Ordering};
use rust_esprit::hal::GpioMode;
use rust_esprit::{HalGpioPin, Pin};

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

/// Last commanded state. The line itself is not readable through the level
/// shifter, so `is_asserted()` answers from here (what the C++ `SwdReset::_state`
/// memoised before).
static ASSERTED: AtomicBool = AtomicBool::new(false);

/// The board table stores pins as plain numbers and `Pin::from` transmutes, so a
/// number outside the enum would be undefined. `lnPin` stops at `PG15`: a board
/// file past that is a table bug the build should catch.
const _: () = assert!(BOARD.dev[DEV_NRST] >= 0 && BOARD.dev[DEV_NRST] <= Pin::PG15 as i32);

/// The target nRST line of the board this firmware was built for.
fn pin() -> Pin {
    Pin::from(BOARD.dev[DEV_NRST] as u32)
}

/// Boot-time setup: put the released level in the latch, then make the pin an
/// output. Same order as the C++ (the latch holds the port's reset value until it
/// is written, so the mode has to come last).
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
