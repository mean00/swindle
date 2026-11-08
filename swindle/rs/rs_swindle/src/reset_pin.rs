//! Target hardware reset pin (nRST) control.
//!
//! Two halves:
//!
//!   * the hardware half is the probe board's — which pin, which polarity, and the
//!     GPIO write through the esprit `GpioPin` trait — in `rs_swindle_board`
//!     (`ln::reset_pin` for the GD32/CH32 probes, `rp2xxx::reset_pin` for the
//!     RP2040/RP2350 ones): no C++ layer between a reset and the GPIO any more;
//!   * the policy half is here: `mon enable_reset_pin`, the pulse / hold-off
//!     delays, the public `ResetPin` API and the C ABI the firmware calls.
//!
//! The ESP32 has no board crate in the build, so this module falls back to the C++
//! `SwdReset` through its two hooks; a hosted build forwards to a probe over RPC.
//! All of it is a GPIO write per reset sequence: nothing here is time critical and
//! nothing is inlined.

/// Probes with a board table: the pin, its polarity and the GPIO write are the
/// board crate's (`rs_swindle_board::reset_pin`); what stays here is *when* the
/// line moves (gate, delays, C ABI).
#[cfg(all(
    feature = "native",
    any(
        feature = "bluepill48",
        feature = "bluepill64",
        feature = "rp_zero",
        feature = "rp_carrier"
    )
))]
mod imp {
    use rs_swindle_board::reset_pin as line;

    /// Boot-time setup of the line.
    pub fn init() {
        line::init();
    }

    /// Drive the line: no `mon enable_reset_pin` check and no delay.
    pub fn set(asserted: bool) {
        line::set(asserted);
    }

    /// Whether the line is currently asserted.
    pub fn is_asserted() -> bool {
        line::is_asserted()
    }

    /// C ABI: boot-time setup of the target nRST line (platform tap).
    #[unsafe(no_mangle)]
    pub extern "C" fn reset_pin_init() {
        init();
    }

    /// C ABI: drive the line without the `swindle_nrst_set_val` delays.
    ///
    /// The platform taps (init / session begin / session end) and the SDI
    /// template's reset pulse use this. The blackmagic C code uses
    /// `platform_nrst_set_val` (`commands/mon.rs`) instead, which honours
    /// `mon enable_reset_pin`.
    #[unsafe(no_mangle)]
    pub extern "C" fn platform_nrst_set_val_internal(assert: bool) {
        set(assert);
    }

    /// C ABI: is the target nRST line asserted? (`cortexm.c`, `command.c`, …)
    #[unsafe(no_mangle)]
    pub extern "C" fn platform_nrst_get_val() -> bool {
        is_asserted()
    }
}

/// Every other build (ESP32, hosted): the C++ `SwdReset` owns the pin
/// (`swindle/src/bmp_nrst_c.cpp`), or the hosted stubs forward to a probe
/// (`hosted/c_stubs.rs`).
#[cfg(not(all(
    feature = "native",
    any(
        feature = "bluepill48",
        feature = "bluepill64",
        feature = "rp_zero",
        feature = "rp_carrier"
    )
)))]
mod imp {
    unsafe extern "C" {
        fn platform_nrst_set_val_internal(assert: bool);
        fn platform_nrst_get_val() -> bool;
    }

    /// The platform tap runs `pReset->setup()` itself.
    pub fn init() {}

    /// Drive the line: no `mon enable_reset_pin` check and no delay.
    pub fn set(asserted: bool) {
        unsafe { platform_nrst_set_val_internal(asserted) }
    }

    /// Whether the line is currently asserted.
    pub fn is_asserted() -> bool {
        unsafe { platform_nrst_get_val() }
    }
}

/// Gated entry point: `mon enable_reset_pin` plus the configured pulse / hold-off
/// delay, i.e. `commands/mon.rs::swindle_nrst_set_val`.
#[cfg(feature = "native")]
fn set_gated(asserted: bool) {
    crate::commands::mon::platform_nrst_set_val(asserted as u32);
}

// Hosted build: the probe does the work, over RPC (`hosted/c_stubs.rs`).
#[cfg(not(feature = "native"))]
unsafe extern "C" {
    #[link_name = "platform_nrst_set_val"]
    fn platform_nrst_set_val_c(assert: bool);
}

#[cfg(not(feature = "native"))]
fn set_gated(asserted: bool) {
    unsafe { platform_nrst_set_val_c(asserted) }
}

/// Target reset pin driver.
pub struct ResetPin;

impl ResetPin {
    /// Assert or release the physical nRST pin.
    ///
    /// When `asserted` is true, the reset line is driven active (held low).
    /// When `asserted` is false, the reset line is released (floats high / pulled up).
    /// Honours `mon enable_reset_pin` and the configured delays.
    pub fn set_asserted(asserted: bool) {
        set_gated(asserted);
    }

    /// Assert the reset line (hold target in reset).
    pub fn assert() {
        Self::set_asserted(true);
    }

    /// Release the reset line (allow target to run).
    pub fn release() {
        Self::set_asserted(false);
    }

    /// Check if target nRST is currently asserted.
    pub fn is_asserted() -> bool {
        imp::is_asserted()
    }

    /// Pulse the reset pin for `duration_ms` then release.
    pub fn pulse(duration_ms: u32) {
        Self::assert();
        crate::target::delay_ms(duration_ms);
        Self::release();
    }

    /// Direct low-level reset control: drives the pin straight away, with no
    /// `mon enable_reset_pin` check and no delay (`mon set_reset_pin`).
    pub fn set_internal(asserted: bool) {
        imp::set(asserted);
    }
}

/// Boot-time setup of the target nRST line (called by the platform tap).
pub fn init() {
    imp::init();
}

/// Drive the target nRST pin (`true` = hold in reset, `false` = release).
pub fn set(asserted: bool) {
    ResetPin::set_asserted(asserted);
}

/// Assert the target nRST pin.
pub fn assert() {
    ResetPin::assert();
}

/// Release the target nRST pin.
pub fn release() {
    ResetPin::release();
}

/// Query whether the target nRST pin is currently asserted.
pub fn is_asserted() -> bool {
    ResetPin::is_asserted()
}

/// Pulse the reset pin for `duration_ms`.
pub fn pulse(duration_ms: u32) {
    ResetPin::pulse(duration_ms);
}
