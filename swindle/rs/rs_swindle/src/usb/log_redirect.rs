//! `swindleRedirectLog_c` - the C entry `mon redirect` calls (`usb.md` P4).
//!
//! This was `swindle/rs/rs_swindle/c_interface/bmp_interface_usb.cpp`: it installs one
//! of the Rust CDC writers as the logger sink and clears it again. Nothing in it was
//! C++ except the file, so it is a Rust export now.
//!
//! Which writer it installs depends on the CDC layout: a 3-CDC build has a dedicated
//! logger interface (`rn_usb_cdc_logger`), a 2-CDC build shares the UART bridge's
//! (`rn_serial_bridge_write`).

use core::ffi::c_char;

/// `lnLoggerFunction` of `logger_c.rs`.
type LoggerFn = Option<unsafe extern "C" fn(n: i32, data: *const c_char)>;

unsafe extern "C" {
    /// Exported by `rust_esprit::logger_c`.
    fn setLogger(f: LoggerFn);
    /// `printf`-style logger, also from `rust_esprit::logger_c`.
    fn Logger(fmt: *const c_char, ...);
    /// The two CDC sinks this module can install (same shape, `u8` instead of `c_char`).
    /// The `_nb` variants: a log sink must never wait for a host to drain the FIFO.
    #[cfg(feature = "cdc3")]
    fn rn_usb_cdc_logger_nb(n: i32, data: *const u8);
    #[cfg(all(not(feature = "cdc3"), feature = "cdc2"))]
    fn rn_serial_bridge_write_nb(n: i32, data: *const u8);
}

/// Adapter to `LoggerFn`: the writers take bytes, the logger hands out chars.
#[cfg(feature = "cdc3")]
unsafe extern "C" fn logger_cdc_sink(n: i32, data: *const c_char) {
    unsafe { rn_usb_cdc_logger_nb(n, data as *const u8) }
}

#[cfg(all(not(feature = "cdc3"), feature = "cdc2"))]
unsafe extern "C" fn bridge_sink(n: i32, data: *const c_char) {
    unsafe { rn_serial_bridge_write_nb(n, data as *const u8) }
}

#[cfg(feature = "cdc3")]
fn sink() -> LoggerFn {
    Some(logger_cdc_sink)
}
#[cfg(all(not(feature = "cdc3"), feature = "cdc2"))]
fn sink() -> LoggerFn {
    Some(bridge_sink)
}
#[cfg(not(any(feature = "cdc2", feature = "cdc3")))]
fn sink() -> LoggerFn {
    None
}

/// `void swindleRedirectLog_c(int32_t toggle)` - point the logger at the probe's CDC,
/// or back at the UART.
#[unsafe(no_mangle)]
pub extern "C" fn swindleRedirectLog_c(toggle: i32) {
    const MSG: &[u8] = b"Setting redirect to usb to %d\n\0";
    unsafe {
        Logger(MSG.as_ptr() as *const c_char, toggle);
        if toggle != 0 {
            setLogger(sink());
        } else {
            setLogger(None);
        }
        Logger(MSG.as_ptr() as *const c_char, toggle);
    }
}
