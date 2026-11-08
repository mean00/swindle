// bmp_native.rs — Native-only BMP C FFI wrappers
// These functions are only used by the native (embedded) rpc_target code.
// They are re-exported from crate::bmp when not(feature = "hosted").

use crate::platform::wire_error::{WireError, WireResult};

unsafe extern "C" {
    fn swindleRedirectLog_c(onoff: bool);
    fn platform_target_clk_output_enable(enable: bool);
    fn rv_dm_start_c();
    fn bmp_rv_dm_read_status_c(adr: u8, value: *mut u32) -> u8;
    fn bmp_rv_dm_write_status_c(adr: u8, value: u32) -> u8;
}

pub fn swindleRedirectLog(onoff: bool) {
    unsafe {
        swindleRedirectLog_c(onoff);
    }
}

pub fn bmp_platform_nrst_set_val(ass: bool) {
    crate::reset_pin::set(ass);
}

pub fn bmp_platform_nrst_get_val() -> bool {
    crate::reset_pin::is_asserted()
}

pub fn bmp_platform_target_clk_output_enable(enable: bool) {
    unsafe { platform_target_clk_output_enable(enable) }
}

/// Read an AP register.
///
/// The return value cannot carry a fault: it is latched in the platform layer, so
/// a failed access reads as 0 here and callers that care query
/// [`bmp_get_dp_fault`] (or use the RPC `ap_read` verb, which reports it).
pub fn bmp_adiv5_ap_read(device_index: u32, ap_selection: u32, address: u32) -> u32 {
    let mut dp = crate::arm::adi::adiv5::DebugPort::new(crate::arm::adi::swd::HardwareSwd);
    dp.dev_index = device_index as u8;
    dp.ap_read(ap_selection as u8, address as u16).unwrap_or(0)
}

pub fn bmp_clear_dp_fault() {
    crate::platform::swd::clear_fault();
}

pub fn bmp_get_dp_fault() -> u32 {
    crate::platform::swd::fault()
}

pub fn bmp_adiv5_mem_read(
    device_index: u32,
    ap_selection: u32,
    csw: u32,
    address: u32,
    buffer: &mut [u8],
) -> i32 {
    let mut dp = crate::arm::adi::adiv5::DebugPort::new(crate::arm::adi::swd::HardwareSwd);
    dp.dev_index = device_index as u8;
    let mut ap = crate::arm::adi::adiv5::AccessPort::new(&mut dp, ap_selection as u8);
    ap.csw = csw;
    let res = ap.mem_read_bytes(buffer, address);
    if res.is_err() && dp.fault == 0 {
        dp.fault = 1;
    }
    dp.fault as i32
}

pub fn bmp_adiv5_mem_write(
    device_index: u32,
    ap_selection: u32,
    csw: u32,
    address: u32,
    align: u32,
    buffer: &[u8],
) -> i32 {
    let mut dp = crate::arm::adi::adiv5::DebugPort::new(crate::arm::adi::swd::HardwareSwd);
    dp.dev_index = device_index as u8;
    let mut ap = crate::arm::adi::adiv5::AccessPort::new(&mut dp, ap_selection as u8);
    ap.csw = csw;
    let res = ap.mem_write_aligned(address, buffer, align);
    if res.is_err() && dp.fault == 0 {
        dp.fault = 1;
    }
    dp.fault as i32
}

pub fn bmp_adiv5_ap_write(device_index: u32, ap_selection: u32, address: u32, value: u32) {
    let mut dp = crate::arm::adi::adiv5::DebugPort::new(crate::arm::adi::swd::HardwareSwd);
    dp.dev_index = device_index as u8;
    let _ = dp.ap_write(ap_selection as u8, address as u16, value);
}

pub fn bmp_adiv5_full_dp_low_level(
    device_index: u32,
    ap_selection: u32,
    address: u16,
    value: u32,
) -> (i32, u32) {
    let mut dp = crate::arm::adi::adiv5::DebugPort::new(crate::arm::adi::swd::HardwareSwd);
    dp.dev_index = device_index as u8;
    use crate::arm::adi::swd::SwdTransport;
    let out = dp.transport.raw_access(ap_selection as u8, address, value);
    (dp.wire_fault() as i32, out)
}

pub fn bmp_adiv5_full_dp_read(device_index: u32, _ap_selection: u32, address: u16) -> (i32, u32) {
    let mut dp = crate::arm::adi::adiv5::DebugPort::new(crate::arm::adi::swd::HardwareSwd);
    dp.dev_index = device_index as u8;
    use crate::arm::adi::swd::SwdTransport;
    let val = dp.transport.dp_read(address);
    (dp.wire_fault() as i32, val)
}

pub fn rv_dm_start() {
    unsafe {
        rv_dm_start_c();
    }
}

pub fn try_rv_read(adr: u8) -> WireResult<u32> {
    let mut ret: u32 = 0;
    let status = unsafe { bmp_rv_dm_read_status_c(adr, &mut ret) };
    WireError::into_read_result(status, ret)
}

pub fn try_rv_write(adr: u8, data: u32) -> WireResult<()> {
    let status = unsafe { bmp_rv_dm_write_status_c(adr, data) };
    WireError::check(status)
}

pub fn bmp_rv_read(adr: u8) -> (bool, u32) {
    match try_rv_read(adr) {
        Ok(val) => (true, val),
        Err(_) => (false, 0),
    }
}

pub fn bmp_rv_write(adr: u8, data: u32) -> bool {
    try_rv_write(adr, data).is_ok()
}
