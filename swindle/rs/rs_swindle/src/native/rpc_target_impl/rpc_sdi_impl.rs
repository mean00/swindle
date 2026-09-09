// rpc_sdi_impl.rs — SDI (WCH CH32V0xx single-wire) class command implementations
// Called by rpc_target_generated.rs

use crate::bmp;

/// Enter SDI debug mode (upload PIO, NRST pulse, 0x7e/0x7d unlock)
pub fn reset() -> (bool, u32) {
    bmp::sdi_dm_start();
    (true, 0)
}

/// Read one SDI DM register
pub fn dm_read(address: u32) -> (bool, u32) {
    bmp::bmp_sdi_read(address as u8)
}

/// Write one SDI DM register
pub fn dm_write(address: u32, value: u32) -> (bool, u32) {
    let ok = bmp::bmp_sdi_write(address as u8, value);
    (ok, 0)
}