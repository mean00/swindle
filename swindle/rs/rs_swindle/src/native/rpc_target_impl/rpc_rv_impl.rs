// rpc_rv_impl.rs — RV class command implementations
// Called by rpc_target_generated.rs

use crate::bmp;

/// Reset RISC-V DM
pub fn reset() -> (bool, u32) {
    bmp::rv_dm_start();
    (true, 0)
}

/// Read RISC-V DM register
pub fn dm_read(address: u32) -> (bool, u32) {
    bmp::bmp_rv_read(address as u8)
}

/// Write RISC-V DM register
pub fn dm_write(address: u32, value: u32) -> (bool, u32) {
    let ok = bmp::bmp_rv_write(address as u8, value);
    (ok, 0)
}