//! Low-level SWD interface provided by the platform driver.

use super::wire_error::{WireError, WireResult};

#[cfg(not(feature = "hosted"))]
unsafe extern "C" {
    fn ln_adiv5_swd_write_no_check(addr: u16, data: u32) -> bool;
    fn ln_adiv5_swd_read_no_check(addr: u16) -> u32;
    fn ln_adiv5_swd_write_checked_c(addr: u16, data: u32) -> u8;
    fn ln_adiv5_swd_read_checked_c(addr: u16, out_data: *mut u32) -> u8;
    fn ln_raw_swd_write(tick: u32, value: u32);
    fn swdptap_init();
    fn bmp_gpio_reset();
    fn bmp_adiv5_swd_raw_access_c(rnw: u8, addr: u16, value: u32, fault: *mut u32) -> u32;
    fn bmp_clear_dp_fault_c();
    fn bmp_get_dp_fault_c() -> u32;
}

#[cfg(feature = "hosted")]
static LAST_FAULT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

/// Initialize SWD GPIO lines and timings.
#[inline]
pub fn init() {
    #[cfg(not(feature = "hosted"))]
    unsafe { swdptap_init(); }
    #[cfg(feature = "hosted")]
    let _ = crate::hosted::rpc_host_generated::remote_swdp_init();
}

/// Reset SWD GPIO pin state (float/pull up SWDIO).
#[inline]
pub fn gpio_reset() {
    #[cfg(not(feature = "hosted"))]
    unsafe { bmp_gpio_reset(); }
    #[cfg(feature = "hosted")]
    let _ = crate::hosted::rpc_host_generated::remote_gen_gpio_reset();
}

/// Write to an SWD register without retry, returning an explicit [`WireResult`].
#[inline]
pub fn try_write_no_check(addr: u16, data: u32) -> WireResult<()> {
    #[cfg(not(feature = "hosted"))]
    {
        let status = unsafe { ln_adiv5_swd_write_checked_c(addr, data) };
        WireError::check(status)
    }
    #[cfg(feature = "hosted")]
    {
        let failed = crate::hosted::rpc_host::remote_rpc::remote_adiv5_swd_write_no_check_rs(addr, data);
        if failed { Err(WireError::Fault) } else { Ok(()) }
    }
}

/// Raw write to an SWD register without retry (`true` = fault, matching C ABI).
#[inline]
pub fn write_no_check(addr: u16, data: u32) -> bool {
    #[cfg(not(feature = "hosted"))]
    unsafe { ln_adiv5_swd_write_no_check(addr, data) }
    #[cfg(feature = "hosted")]
    crate::hosted::rpc_host::remote_rpc::remote_adiv5_swd_write_no_check_rs(addr, data)
}

/// Read from an SWD register without retry, returning an explicit [`WireResult<u32>`].
#[inline]
pub fn try_read_no_check(addr: u16) -> WireResult<u32> {
    #[cfg(not(feature = "hosted"))]
    {
        let mut data = 0u32;
        let status = unsafe { ln_adiv5_swd_read_checked_c(addr, &mut data) };
        WireError::into_read_result(status, data)
    }
    #[cfg(feature = "hosted")]
    {
        let val = crate::hosted::rpc_host::remote_rpc::remote_adiv5_swd_read_no_check_rs(addr);
        if val == 0 {
            Err(WireError::NoResponse)
        } else {
            Ok(val)
        }
    }
}

/// Raw read from an SWD register without retry (returns `0` on ACK/parity fault).
#[inline]
pub fn read_no_check(addr: u16) -> u32 {
    #[cfg(not(feature = "hosted"))]
    unsafe { ln_adiv5_swd_read_no_check(addr) }
    #[cfg(feature = "hosted")]
    crate::hosted::rpc_host::remote_rpc::remote_adiv5_swd_read_no_check_rs(addr)
}

/// Output `ticks` bits with value `value` on SWD lines.
#[inline]
pub fn raw_write(ticks: u32, value: u32) {
    #[cfg(not(feature = "hosted"))]
    unsafe { ln_raw_swd_write(ticks, value); }
    #[cfg(feature = "hosted")]
    crate::hosted::rpc_host::remote_rpc::remote_raw_swd_write_rs(ticks, value);
}

/// Execute a raw SWD access (read or write), returning `(response, fault)`.
#[inline]
pub fn raw_access_with_fault(rnw: u8, addr: u16, value: u32) -> (u32, u32) {
    #[cfg(not(feature = "hosted"))]
    {
        let mut fault = 0u32;
        let resp = unsafe { bmp_adiv5_swd_raw_access_c(rnw, addr, value, &mut fault) };
        (resp, fault)
    }
    #[cfg(feature = "hosted")]
    {
        let mut fault = 0u32;
        let resp = crate::hosted::rpc_host::remote_rpc::remote_adiv5_swd_raw_access_rs(rnw, addr, value, &mut fault);
        LAST_FAULT.store(fault, core::sync::atomic::Ordering::Relaxed);
        (resp, fault)
    }
}

/// Execute a raw SWD access (read or write), returning an explicit [`WireResult<u32>`].
#[inline]
pub fn try_raw_access(rnw: u8, addr: u16, value: u32) -> WireResult<u32> {
    let (resp, fault) = raw_access_with_fault(rnw, addr, value);
    if fault == 0 {
        Ok(resp)
    } else {
        WireError::into_read_result(fault as u8, resp)
    }
}

/// Execute a raw SWD access (read or write).
#[inline]
pub fn raw_access(rnw: u8, addr: u16, value: u32) -> u32 {
    raw_access_with_fault(rnw, addr, value).0
}

/// Query wire fault status.
#[inline]
pub fn fault() -> u32 {
    #[cfg(not(feature = "hosted"))]
    unsafe { bmp_get_dp_fault_c() }
    #[cfg(feature = "hosted")]
    LAST_FAULT.load(core::sync::atomic::Ordering::Relaxed)
}

/// Clear wire fault status.
#[inline]
pub fn clear_fault() {
    #[cfg(not(feature = "hosted"))]
    unsafe { bmp_clear_dp_fault_c(); }
    #[cfg(feature = "hosted")]
    LAST_FAULT.store(0, core::sync::atomic::Ordering::Relaxed);
}
