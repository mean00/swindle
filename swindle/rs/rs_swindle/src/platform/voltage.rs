//! Target sense voltage monitoring.

use core::ffi::c_char;
use heapless::String;

#[cfg(feature = "native")]
use rust_esprit::{Adc, AdcConfig};

/// Conversions averaged into one voltage reading: the sense divider is high
/// impedance, so a handful of readings removes most of the SAR noise.
const VOLTAGE_AVERAGING: u8 = 16;

/// Acquisition window asked of the ADC: 4 µs covers the 239.5-cycle `SMPT` code of
/// the GD32 at 72 MHz, and any shorter window a family offers. The RP2xxx ignores
/// it (its window is fixed by silicon).
const SENSE_SAMPLE_TIME_NS: u32 = 4_000;

/// Measure voltage using any driver implementing the generic [`Adc`] trait.
#[cfg(feature = "native")]
pub fn measure_voltage<A: Adc>(adc: &mut A, multiplier: f32) -> f32 {
    let mut sample = 0i32;
    for _ in 0..VOLTAGE_AVERAGING {
        sample += adc.read().unwrap_or(0) as i32;
    }
    let avg = (sample as f32) / VOLTAGE_AVERAGING as f32;
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
        let mut adc = rust_esprit::AdcDriver::new(0, pin);
        // The sense divider is high impedance: the longest acquisition window the
        // backend can give us, and the averaging of `measure_voltage` on top.
        let _ = adc.configure(&AdcConfig {
            sample_time_ns: SENSE_SAMPLE_TIME_NS,
            averaging: 1,
        });
        measure_voltage(&mut adc, mult)
    }
    #[cfg(not(feature = "native"))]
    {
        0.0
    }
}

/// Read target voltage as formatted string (e.g. "3.30 v"), if available.
pub fn target_voltage_str() -> Option<String<16>> {
    #[cfg(feature = "native")]
    {
        let v = target_voltage_volts();
        if v == 0.0 {
            return None;
        }
        let by_100 = (v * 100.0) as u32;
        let left = by_100 / 100;
        let right = by_100 % 100;

        let mut s = String::<16>::new();
        // Format "{left}.{right:02} v" without core::fmt
        let mut buf = [0u8; 10];
        let mut n = 0;
        let mut v_left = left;
        loop {
            buf[n] = b'0' + (v_left % 10) as u8;
            n += 1;
            v_left /= 10;
            if v_left == 0 {
                break;
            }
        }
        while n > 0 {
            n -= 1;
            let _ = s.push(buf[n] as char);
        }
        let _ = s.push('.');
        let _ = s.push((b'0' + (right / 10) as u8) as char);
        let _ = s.push((b'0' + (right % 10) as u8) as char);
        let _ = s.push_str(" v");
        Some(s)
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
                let cstr = core::ffi::CStr::from_ptr(ptr);
                cstr.to_str().ok().and_then(|str_slice| {
                    let mut s = String::<16>::new();
                    s.push_str(str_slice).ok()?;
                    Some(s)
                })
            }
        }
    }
}

#[cfg(feature = "native")]
#[unsafe(no_mangle)]
pub extern "C" fn platform_target_voltage() -> *const c_char {
    static C_BUF: crate::SyncCell<[u8; 16]> = crate::SyncCell::new([0u8; 16]);
    match target_voltage_str() {
        Some(s) => {
            let bytes = s.as_bytes();
            let len = bytes.len().min(15);
            // SAFETY: Single-threaded C export called synchronously.
            unsafe {
                let buf = C_BUF.as_mut();
                buf[..len].copy_from_slice(&bytes[..len]);
                buf[len] = 0;
                buf.as_ptr() as *const c_char
            }
        }
        None => core::ptr::null(),
    }
}
