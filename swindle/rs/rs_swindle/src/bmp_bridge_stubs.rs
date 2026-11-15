//! Platform glue stubs for Black Magic Debug (`swindle/src/bridge.cpp` replacement).
//!
//! Replaces `swindle/src/bridge.cpp` in `LN_ENABLE_RUST` native builds.

#[repr(C)]
pub struct PlatformTimeout {
    pub time: u32,
}

unsafe extern "C" {
    #[link_name = "\u{1}_Z18bmp_gpio_init_oncev"]
    fn bmp_gpio_init_once();
}

#[unsafe(no_mangle)]
pub extern "C" fn pins_init() {
    unsafe { bmp_gpio_init_once() };
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_init() {}

#[unsafe(no_mangle)]
pub extern "C" fn platform_get_adc_nreset_pin() -> u32 {
    #[cfg(any(feature = "bluepill48", feature = "bluepill64", feature = "rp_carrier"))]
    {
        rs_swindle_board::BOARD.adc_nreset.unwrap_or(0) as u32
    }
    #[cfg(not(any(feature = "bluepill48", feature = "bluepill64", feature = "rp_carrier")))]
    {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_get_adc_nreset_multiplier() -> f32 {
    #[cfg(any(feature = "bluepill48", feature = "bluepill64", feature = "rp_carrier"))]
    {
        rs_swindle_board::BOARD.adc_multiplier
    }
    #[cfg(not(any(feature = "bluepill48", feature = "bluepill64", feature = "rp_carrier")))]
    {
        1.0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_time_ms() -> u32 {
    rust_esprit::time_ms()
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_delay(ms: u32) {
    rust_esprit::delay_ms(ms);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn platform_timeout_set(t: *mut PlatformTimeout, ms: u32) {
    unsafe {
        (*t).time = rust_esprit::time_ms().wrapping_add(ms);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn platform_timeout_is_expired(t: *const PlatformTimeout) -> bool {
    let now = rust_esprit::time_ms();
    let target = unsafe { (*t).time };
    (now.wrapping_sub(target) as i32) >= 0
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_target_clk_output_enable(_enable: bool) {}

const BOARDS_ENABLED_CSTR: &str = match option_env!("LN_BOARDS_ENABLED") {
    Some(s) => s,
    None => "",
};

// Keep a static NUL-terminated buffer or build it in const via bytes?
// Since `concat!` only accepts literals, we format into a small const byte buffer.
const fn make_boards_cstr(s: &str) -> [u8; 512] {
    let bytes = s.as_bytes();
    let mut buf = [0u8; 512];
    let len = if bytes.len() < 511 { bytes.len() } else { 511 };
    let mut i = 0;
    while i < len {
        buf[i] = bytes[i];
        i += 1;
    }
    buf[len] = 0;
    buf
}

static BOARDS_ENABLED_BUF: [u8; 512] = make_boards_cstr(BOARDS_ENABLED_CSTR);

#[unsafe(no_mangle)]
pub extern "C" fn list_enabled_boards() -> *const core::ffi::c_char {
    BOARDS_ENABLED_BUF.as_ptr() as *const core::ffi::c_char
}
