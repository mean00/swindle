#[cfg(all(not(feature = "hosted"), not(feature = "network")))]
mod usb_gdb;
#[cfg(all(not(feature = "hosted"), not(feature = "network")))]
mod usb_serial_bridge;
/// The probe's USB device (CDC count vs the MCU endpoint budget) - usb.md P4.
#[cfg(all(not(feature = "hosted"), not(feature = "network"), feature = "usb_stack"))]
mod board;
/// `swindleRedirectLog_c` - usb.md P4.
#[cfg(all(not(feature = "hosted"), not(feature = "network"), feature = "usb_stack"))]
mod log_redirect;
/// `goDfu` + the DFU-runtime hook - usb.md P5.
#[cfg(all(not(feature = "hosted"), not(feature = "network"), feature = "usb_stack"))]
mod dfu;
/// GD32 entry point + USB GDB task (`swindle_gd32_ws2812.cpp` + `bmp_usb.cpp`).
#[cfg(all(
    feature = "bluepill",
    feature = "usb_stack",
    not(feature = "hosted"),
    not(feature = "network")
))]
mod gd32_usb;

// Re-exports for swindle.rs (replaces old swindle_usb.rs)
#[cfg(all(not(feature = "hosted"), not(feature = "network")))]
pub(crate) fn rngdb_output_flush() {
    usb_gdb::rngdb_output_flush_c()
}
#[allow(dead_code)]
#[cfg(all(not(feature = "hosted"), not(feature = "network")))]
pub(crate) fn rngdb_send_data(data: &str) {
    usb_gdb::rngdb_send_data_c(data.len() as u32, data.as_ptr())
}
#[cfg(all(not(feature = "hosted"), not(feature = "network")))]
pub(crate) fn rngdb_send_data_u8(data: &[u8]) {
    usb_gdb::rngdb_send_data_c(data.len() as u32, data.as_ptr())
}