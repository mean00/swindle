//! Target sense voltage monitoring.

use core::ffi::c_char;

#[cfg(feature = "native")]
use rust_esprit::HalSimpleAdc;

/// Measure voltage using any driver implementing the generic `SimpleAdc` trait.
#[cfg(feature = "native")]
pub fn measure_voltage<A: HalSimpleAdc>(adc: &A, multiplier: f32) -> f32 {
    let mut sample = 0i32;
    for _ in 0..16 {
        sample += adc.read();
    }
    let avg = (sample as f32) / 16.0;
    (avg * 3.3 * multiplier) / 4095.0
}

#[cfg(feature = "native")]
unsafe extern "C" {
    fn platform_get_adc_nreset_pin() -> u32;
    fn platform_get_adc_nreset_multiplier() -> f32;
}

/// Read target voltage in Volts (C ABI).
#[cfg(feature = "native")]
#[unsafe(no_mangle)]
pub extern "C" fn bmp_get_target_voltage_c() -> f32 {
    target_voltage_volts()
}

/// Read target voltage in Volts.
pub fn target_voltage_volts() -> f32 {
    #[cfg(feature = "native")]
    {
        let pin = unsafe { platform_get_adc_nreset_pin() };
        if pin == 0 {
            return 0.0;
        }
        let mult = unsafe { platform_get_adc_nreset_multiplier() };
        let adc = rust_esprit::simple_adc::SimpleAdc::new(0, pin);
        adc.set_smpt(7);
        measure_voltage(&adc, mult)
    }
    #[cfg(not(feature = "native"))]
    {
        0.0
    }
}

/// Read target voltage as formatted string (e.g. "3.30 v"), if available.
pub fn target_voltage_str() -> Option<&'static str> {
    #[cfg(feature = "native")]
    {
        let v = target_voltage_volts();
        if v == 0.0 {
            return None;
        }
        static mut BUFFER: [u8; 16] = [0u8; 16];
        let by_100 = (v * 100.0) as u32;
        let left = by_100 / 100;
        let right = by_100 % 100;

        unsafe {
            let buf = &mut *core::ptr::addr_of_mut!(BUFFER);
            let mut idx = 0;
            if left >= 10 {
                buf[idx] = b'0' + ((left / 10) as u8);
                idx += 1;
            }
            buf[idx] = b'0' + ((left % 10) as u8);
            idx += 1;
            buf[idx] = b'.';
            idx += 1;
            buf[idx] = b'0' + ((right / 10) as u8);
            idx += 1;
            buf[idx] = b'0' + ((right % 10) as u8);
            idx += 1;
            buf[idx] = b' ';
            idx += 1;
            buf[idx] = b'v';
            idx += 1;
            buf[idx] = 0;

            core::str::from_utf8(&buf[..idx - 1]).ok()
        }
    }
    #[cfg(not(feature = "native"))]
    {
        unsafe extern "C" {
            fn platform_target_voltage() -> *const c_char;
        }
        unsafe {
            let ptr = platform_target_voltage();
            if ptr.is_null() {
                None
            } else {
                core::ffi::CStr::from_ptr(ptr).to_str().ok()
            }
        }
    }
}

#[cfg(feature = "native")]
#[unsafe(no_mangle)]
pub extern "C" fn platform_target_voltage() -> *const c_char {
    static mut C_BUF: [u8; 16] = [0u8; 16];
    match target_voltage_str() {
        Some(s) => {
            let bytes = s.as_bytes();
            let len = bytes.len().min(15);
            unsafe {
                let buf = &mut *core::ptr::addr_of_mut!(C_BUF);
                buf[..len].copy_from_slice(&bytes[..len]);
                buf[len] = 0;
                buf.as_ptr() as *const c_char
            }
        }
        None => core::ptr::null(),
    }
}
