// rpc_lnadiv_impl.rs — LNADIV class command implementations
// Called by rpc_target_generated.rs

use crate::bmp;

/// SWD write (no check)
pub fn write(address: u32, data: u32) -> (bool, u32) {
    let ok = bmp::bmp_adiv5_swd_write_no_check(address as u16, data);
    (ok, 0)
}

/// SWD read (no check)
pub fn read(address: u32) -> (bool, u32) {
    let value = bmp::bmp_adiv5_swd_read_no_check(address as u16);
    (true, value)
}

/// Raw SWD write
pub fn raw_write(tick_count: u32, value: u32) -> (bool, u32) {
    bmp::bmp_raw_swd_write(tick_count, value);
    (true, 0)
}

/// Low-level SWD access
pub fn lowlevel(rnw: u32, address: u32, value: u32) -> (u32, u32) {
    let mut fault: u32 = 0;
    let response = bmp::bmp_adiv5_swd_raw_access(rnw as u8, address as u16, value, &mut fault);
    (response, fault)
}