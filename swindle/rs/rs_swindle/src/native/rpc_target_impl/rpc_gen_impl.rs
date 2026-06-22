// rpc_gen_impl.rs — GEN class command implementations
// Called by rpc_target_generated.rs

use crate::bmp;

/// Start session — returns "LNBMP"
pub fn start() -> &'static [u8] {
    b"LNBMP"
}
static mut vol: [u8; 5] = [0; 5];
/// Get voltage — returns "????"
pub fn voltage() -> &'static [u8] {
    let r = unsafe { &mut vol };
    let mut tail: usize = 0;
    let voltage: f32 = bmp::bmp_get_target_voltage();
    let left: u32 = (voltage) as u32;
    let right: u8 = ((voltage - (left as f32)) * 10.) as u8;
    let mul = (left / 10) as u8;
    let digit: u8 = left as u8 - 10 * mul;
    if mul > 0 {
        r[0] = b'0' + mul;
        tail = 1;
    }
    r[tail] = b'0' + digit;
    tail += 1;
    r[tail] = b'.';
    tail += 1;
    r[tail] = b'0' + right;
    tail += 1;
    unsafe { &vol[0..tail] }
}

/// Set NRST
pub fn nrst_set(assert: bool) {
    bmp::bmp_platform_nrst_set_val(assert);
}

/// Get NRST
pub fn nrst_get() -> u32 {
    bmp::bmp_platform_nrst_get_val() as u32
}

/// Enable/disable target clock output
pub fn target_clk_oe(enable: bool) {
    bmp::bmp_platform_target_clk_output_enable(enable);
}

/// Set frequency (no-op for now)
pub fn freq_set(freq: u32) {
    crate::native::rpc_target_impl::rpc_swindle_impl::set_fq(freq);
    // no-op
}

/// Get frequency
pub fn freq_get() -> u32 {
    let _ok: bool;
    let fq: u32;
    (_ok, fq) = crate::native::rpc_target_impl::rpc_swindle_impl::get_fq();
    fq
}

/// Set power (not supported)
pub fn pwr_set(_enabled: bool) {
    // not supported
}

/// Get power (not supported)
pub fn pwr_get() -> u32 {
    0 // not supported
}

/// Reset SWDIO GPIO state (set high + output).
///
/// Drives the SWDIO pin high and configures it as an output. This is called
/// during target detach to release the SWD bus to a known idle state.
pub fn gpio_reset() {
    bmp::bmp_gpio_reset();
}
