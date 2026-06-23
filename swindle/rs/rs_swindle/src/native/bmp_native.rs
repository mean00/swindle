// bmp_native.rs — Native-only BMP C FFI wrappers
// These functions are only used by the native (embedded) rpc_target code.
// They are re-exported from crate::bmp when not(feature = "hosted").

use crate::rn_bmp_cmd_c;

pub fn rpc_init_swd() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_rpc_init_swd_c() }
}

pub fn swindleRedirectLog(onoff: bool) {
    unsafe {
        rn_bmp_cmd_c::swindleRedirectLog_c(onoff);
    }
}

pub fn bmp_platform_nrst_set_val(ass: bool) {
    unsafe {
        rn_bmp_cmd_c::platform_nrst_set_val(ass);
    }
}

pub fn bmp_platform_nrst_get_val() -> bool {
    unsafe { rn_bmp_cmd_c::platform_nrst_get_val() }
}

pub fn bmp_platform_target_clk_output_enable(enable: bool) {
    unsafe { rn_bmp_cmd_c::platform_target_clk_output_enable(enable) }
}

pub fn bmp_adiv5_ap_read(device_index: u32, ap_selection: u32, address: u32) -> u32 {
    unsafe { rn_bmp_cmd_c::bmp_adiv5_ap_read_c(device_index, ap_selection, address) }
}

pub fn bmp_clear_dp_fault() {
    unsafe {
        rn_bmp_cmd_c::bmp_clear_dp_fault_c();
    }
}

pub fn bmp_adiv5_mem_read(
    device_index: u32,
    ap_selection: u32,
    csw: u32,
    address: u32,
    buffer: &mut [u8],
) -> i32 {
    unsafe {
        rn_bmp_cmd_c::bmp_adiv5_mem_read_c(
            device_index,
            ap_selection,
            csw,
            address,
            buffer.as_ptr() as *mut u8,
            buffer.len() as u32,
        )
    }
}

pub fn bmp_adiv5_mem_write(
    device_index: u32,
    ap_selection: u32,
    csw: u32,
    address: u32,
    align: u32,
    buffer: &[u8],
) -> i32 {
    unsafe {
        rn_bmp_cmd_c::bmp_adiv5_mem_write_c(
            device_index,
            ap_selection,
            csw,
            address,
            align,
            buffer.as_ptr(),
            buffer.len() as u32,
        )
    }
}

pub fn bmp_adiv5_ap_write(device_index: u32, ap_selection: u32, address: u32, value: u32) {
    unsafe {
        rn_bmp_cmd_c::bmp_adiv5_ap_write_c(device_index, ap_selection, address, value);
    }
}

pub fn bmp_adiv5_full_dp_low_level(
    device_index: u32,
    ap_selection: u32,
    address: u16,
    value: u32,
) -> (i32, u32) {
    unsafe {
        let mut outvalue: u32 = 0;
        let ptr: *mut u32 = &mut outvalue;

        let mut ret: i32 = -1;
        let ret_ptr: *mut i32 = &mut ret;

        rn_bmp_cmd_c::bmp_adiv5_full_dp_low_level_c(
            device_index,
            ap_selection,
            address,
            value,
            ret_ptr,
            ptr,
        );
        (ret, outvalue)
    }
}

pub fn bmp_adiv5_full_dp_read(device_index: u32, ap_selection: u32, address: u16) -> (i32, u32) {
    unsafe {
        let mut value: u32 = 0;
        let ptr: *mut u32 = &mut value;

        let mut ret: i32 = -1;
        let ret_ptr: *mut i32 = &mut ret;

        rn_bmp_cmd_c::bmp_adiv5_full_dp_read_c(device_index, ap_selection, address, ret_ptr, ptr);
        (ret, value)
    }
}

pub fn rv_dm_start() {
    unsafe {
        rn_bmp_cmd_c::rv_dm_start_c();
    }
}

pub fn bmp_rv_read(adr: u8) -> (bool, u32) {
    unsafe {
        let mut ret: u32 = 0;
        let ret_ptr: *mut u32 = &mut ret;

        let status: bool = rn_bmp_cmd_c::bmp_rv_dm_read_c(adr, ret_ptr);
        (status, ret)
    }
}

pub fn bmp_rv_write(adr: u8, data: u32) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_rv_dm_write_c(adr, data) }
}

pub fn bmp_adiv5_swd_write_no_check(addr: u16, data: u32) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_adiv5_swd_write_no_check_c(addr, data) }
}

pub fn bmp_adiv5_swd_read_no_check(addr: u16) -> u32 {
    unsafe { rn_bmp_cmd_c::bmp_adiv5_swd_read_no_check_c(addr) }
}

pub fn bmp_raw_swd_write(tick: u32, value: u32) {
    unsafe { rn_bmp_cmd_c::bmp_raw_swd_write_c(tick, value) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn bmp_adiv5_swd_raw_access(rnw: u8, addr: u16, value: u32, fault: *mut u32) -> u32 {
    unsafe { rn_bmp_cmd_c::bmp_adiv5_swd_raw_access_c(rnw, addr, value, fault) }
}