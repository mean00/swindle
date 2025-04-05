/*
 *
 */
use crate::rn_bmp_cmd_c::{bmp_rtt_get_info_c, bmp_rtt_set_info_c, rttField, rttInfo};

impl rttInfo {
    pub fn new() -> Self {
        rttInfo {
            enabled: 0,
            found: 0,
            min_address: 0,
            max_address: 0,
            min_poll_ms: 0,
            max_poll_ms: 0,
            max_poll_error: 0,
            cb_address: 0,
        }
    }
}
/**
 * Retrieve all rtt stuff in one go
 */
pub fn get_rtt_info() -> rttInfo {
    let mut info = rttInfo::new();
    unsafe {
        bmp_rtt_get_info_c(&mut info);
    }
    info
}
/**
 * Retrieve all rtt stuff in one go
 */
pub fn set_rtt_info(field: rttField, info: &rttInfo) {
    unsafe {
        bmp_rtt_set_info_c(field, info);
    }
}
// EOF
