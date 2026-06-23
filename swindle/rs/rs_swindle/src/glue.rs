//! C/Rust glue code — GDB console output via `O` packets.
//!
//! Provides `gdb_out()` (C-compatible) and `gdb_out_rs()` (Rust-native) for
//! sending text to the GDB remote console. Both encode the text as a GDB `O`
//! packet (hex-encoded console output).

use crate::encoder::encoder;
use core::ffi::CStr;
//
#[cfg(feature = "hosted")]
type my_c_str = *const i8;
#[cfg(not(feature = "hosted"))]
type my_c_str = *const u8;

/// Send hex-encoded text to the GDB console (C-compatible entry point).
///
/// Called from C code via `extern "C"`. Takes a null-terminated string pointer
/// and sends it as a GDB `O` packet (hex-encoded console output).
#[unsafe(no_mangle)]
pub extern "C" fn gdb_out(fmt: *const u8) {
    let mut e = encoder::new();
    e.begin();
    e.add("O");

    let slice: &str;
    unsafe {
        slice = match CStr::from_ptr(fmt as my_c_str).to_str() {
            Ok(x) => x,
            Err(_y) => return,
        };
    }
    e.hex_and_add(slice);
    e.end();
}

/// Send hex-encoded text to the GDB console (Rust-native entry point).
///
/// Takes a `&str` and sends it as a GDB `O` packet (hex-encoded console output).
pub fn gdb_out_rs(fmt: &str) {
    let mut e = encoder::new();
    e.begin();
    e.add("O");
    e.hex_and_add(fmt);
    e.end();
}
