//! The two `xAssert(0)` stubs of the old `c_interface/bmp_stubs_c.cpp`, in Rust.
//!
//! The C file kept two C ABI symbols defined that the Blackmagic core used to need on
//! this platform: `vasprintf_()` (the `vasprintf` -> `vasprintf_` rename of
//! `swindle/include/platform.h`) and `gdb_putpacket_f()` (the GDB packet formatter).
//! Nothing in the current firmware calls them - the GDB stub and every formatter are
//! Rust (`usb_gdb.rs`, `format_util.rs`) - so they only have to stay *defined*, and
//! they assert exactly like the C did rather than silently passing: a C caller coming
//! back must stop where the old code stopped.
//!
//! Compiled in both halves of the build (the hosted build compiles the same
//! `swindle_interface` target), so `do_assert()` is reached the same way there.

use core::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    /// The probe's panic path, what `xAssert(0)` expands to
    /// (`esprit/include/lnAssert.h`): prints "FATAL ERROR :<text>" and never returns.
    /// Native: `esprit/src/lnFreeRTOS_pp.cpp`. Hosted: `hosted/c_stubs.rs`.
    fn do_assert(text: *const c_char) -> !;
}

/// The C code's `xAssert(0)`: name what was reached, then stop for good.
fn assert_reached(name: &'static [u8]) -> ! {
    unsafe { do_assert(name.as_ptr() as *const c_char) }
}

/// `vasprintf_()`: the Rust side formats into its own buffers, so this is never
/// called - and reaching it is a bug.
///
/// `va_list` has no Rust type; the third argument is the pointer AAPCS passes for it
/// in practice, and it is never read because the function does not return.
#[unsafe(no_mangle)]
pub extern "C" fn vasprintf_(
    _strp: *mut *mut c_char,
    _fmt: *const c_char,
    _ap: *mut c_void,
) -> c_int {
    assert_reached(b"vasprintf_\0")
}

/// `gdb_putpacket_f()`: `usb_gdb.rs` builds the packets, so this is never called
/// either. The C one was variadic; the extra arguments are simply not read.
#[unsafe(no_mangle)]
pub extern "C" fn gdb_putpacket_f(_fmt: *const c_char) -> ! {
    assert_reached(b"gdb_putpacket_f\0")
}
