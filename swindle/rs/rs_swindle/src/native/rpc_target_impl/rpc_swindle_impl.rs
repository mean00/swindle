// rpc_swindle_impl.rs — SWINDLE class command implementations
// Called by rpc_target_generated.rs

use crate::bmp::{bmp_get_frequency, bmp_set_frequency};
use crate::crc::do_local_crc32;

/// Calculate CRC32
pub fn crc32(address: u32, length: u32) -> (bool, u32) {
    do_local_crc32(address, length)
}

/// Get frequency
pub fn get_fq() -> (bool, u32) {
    let fq = bmp_get_frequency();
    (true, fq)
}

/// Set frequency
pub fn set_fq(freq_hz: u32) {
    bmp_set_frequency(freq_hz);
}