//! Clock frequency and wait state control.

unsafe extern "C" {
    fn bmp_set_frequency_c(fq: u32);
    fn bmp_get_frequency_c() -> u32;
    fn bmp_set_wait_state_c(ws: u32);
    fn bmp_get_wait_state_c() -> u32;
}

/// Set the SWD clock frequency in Hz.
#[inline]
pub fn set_frequency(hz: u32) {
    unsafe { bmp_set_frequency_c(hz); }
}

/// Get the current SWD clock frequency in Hz.
#[inline]
pub fn frequency() -> u32 {
    unsafe { bmp_get_frequency_c() }
}

/// Set the wait state delay counter.
#[inline]
pub fn set_wait_states(ws: u32) {
    unsafe { bmp_set_wait_state_c(ws); }
}

/// Get the wait state delay counter.
#[inline]
pub fn wait_states() -> u32 {
    unsafe { bmp_get_wait_state_c() }
}
