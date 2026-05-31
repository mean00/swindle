#[cfg(all(not(feature = "hosted"), not(feature = "network")))]
mod usb_gdb;
#[cfg(all(not(feature = "hosted"), not(feature = "network")))]
mod usb_serial_bridge;

// Re-exports for swindle.rs (replaces old swindle_usb.rs)
#[cfg(all(not(feature = "hosted"), not(feature = "network")))]
pub(crate) fn rngdb_output_flush() {
    usb_gdb::rngdb_output_flush_c()
}
#[cfg(all(not(feature = "hosted"), not(feature = "network")))]
pub(crate) fn rngdb_send_data(data: &str) {
    usb_gdb::rngdb_send_data_c(data.len() as u32, data.as_ptr())
}
#[cfg(all(not(feature = "hosted"), not(feature = "network")))]
pub(crate) fn rngdb_send_data_u8(data: &[u8]) {
    usb_gdb::rngdb_send_data_c(data.len() as u32, data.as_ptr())
}