//! Low-level SWD interface provided by the platform driver.

#[cfg(not(feature = "hosted"))]
unsafe extern "C" {
    fn ln_adiv5_swd_write_no_check(addr: u16, data: u32) -> bool;
    fn ln_adiv5_swd_read_no_check(addr: u16) -> u32;
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

/// Raw write to an SWD register without checking ACK/fault.
#[inline]
pub fn write_no_check(addr: u16, data: u32) -> bool {
    #[cfg(not(feature = "hosted"))]
    unsafe { ln_adiv5_swd_write_no_check(addr, data) }
    #[cfg(feature = "hosted")]
    crate::hosted::rpc_host::remote_rpc::remote_adiv5_swd_write_no_check_rs(addr, data)
}

/// Raw read from an SWD register without checking ACK/fault.
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
