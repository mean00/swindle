use crate::commands::run::HaltState;
use crate::rn_bmp_cmd_c;
use alloc::vec::Vec;
use core::ffi::CStr;
use core::ptr::null;
use core::ptr::null_mut;

#[cfg(feature = "hosted")]
type my_c_str = *const i8;
#[cfg(not(feature = "hosted"))]
type my_c_str = *const u8;

pub enum mapping {
    Flash = 0,
    Ram = 1,
}

pub struct MemoryBlock {
    pub start_address: u32,
    pub length: u32,
    pub block_size: u32,
}

pub fn bmp_register_description() -> &'static str {
    //
    unsafe {
        #![allow(clippy::manual_unwrap_or_default)]
        match CStr::from_ptr(rn_bmp_cmd_c::bmp_target_description_c() as my_c_str).to_str() {
            Ok(x) => x,
            Err(_y) => "",
        }
    }
}
pub fn bmp_drop_register_description() {
    unsafe { rn_bmp_cmd_c::bmp_target_description_clear_c() }
}

pub fn bmp_read_registers() -> Vec<u32> {
    if !bmp_attached() {
        let r: Vec<u32> = Vec::new();
        return r;
    }
    unsafe {
        let n = rn_bmp_cmd_c::bmp_registers_count_c() as usize;
        let mut r: Vec<u32> = vec![0; n];
        r.set_len(n);
        rn_bmp_cmd_c::bmp_read_all_registers_c(r.as_mut_ptr()); // as *mut u32);
        r
    }
}
pub fn bmp_get_mapping(map: mapping) -> Vec<MemoryBlock> {
    if !bmp_attached() {
        let r: Vec<MemoryBlock> = Vec::new();
        return r;
    }
    let mut r: Vec<MemoryBlock> = Vec::new();
    let count: usize;
    let imap = map as u32;
    unsafe {
        count = rn_bmp_cmd_c::bmp_map_count_c(imap) as usize;
    };
    if count != 0 {
        unsafe {
            let mut start: u32 = 0;
            let mut size: u32 = 0;
            let mut block_size: u32 = 0;
            for i in 0..count {
                if rn_bmp_cmd_c::bmp_map_get_c(
                    imap,
                    i as u32,
                    &mut start as *mut u32,
                    &mut size as *mut u32,
                    &mut block_size as *mut u32,
                ) {
                    r.push(MemoryBlock {
                        start_address: start,
                        length: size,
                        block_size,
                    });
                }
            }
        }
    }
    r
}
pub fn swdp_scan() -> bool {
    unsafe { rn_bmp_cmd_c::cmd_swd_scan(null(), 0, null_mut()) }
}
pub fn rvswdp_scan() -> bool {
    unsafe { rn_bmp_cmd_c::cmd_rvswd_scan(null(), 0, null_mut()) }
}
pub fn bmp_detach() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_detach_c(0) }
}

pub fn bmp_get_target_voltage() -> f32 {
    unsafe { rn_bmp_cmd_c::bmp_get_target_voltage_c() }
}

pub fn bmp_attached() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_attached_c() }
}
pub fn bmp_attach(target: u32) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_attach_c(target) }
}
pub fn bmp_write_register(reg: u32, value: u32) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_write_reg_c(reg, value) }
}
pub fn bmp_read_register(reg: u32) -> Option<u32> {
    let mut val: u32 = 0;
    unsafe {
        let ptr: *mut u32 = &mut val;
        if rn_bmp_cmd_c::bmp_read_reg_c(reg, ptr) {
            return Some(val);
        }
        None
    }
}

pub fn riscv_list_csr(out: &mut [u32]) -> Option<&[u32]> {
    let n = unsafe { rn_bmp_cmd_c::riscv_list_csr(0, 0, core::ptr::null_mut()) };
    if n > (out.len() as u32) {
        return None;
    }
    let out_u32: *mut u32 = &mut out[0];
    let r: usize = unsafe { rn_bmp_cmd_c::riscv_list_csr(0, n, out_u32) as usize };
    Some(&out[0..r])
}

pub fn bmp_flash_erase(adr: u32, size: u32) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_flash_erase_c(adr, size) }
}

pub fn bmp_flash_write(addr: u32, data: &[u8]) -> bool {
    unsafe {
        let ptr: *const u8 = data.as_ptr();
        rn_bmp_cmd_c::bmp_flash_write_c(addr, data.len() as u32, ptr)
    }
}
/*
 * Careful it is stongly linked to the name within bmp
 */
pub fn bmp_get_target_name() -> &'static str {
    unsafe {
        #![allow(clippy::manual_unwrap_or_default)]
        match CStr::from_ptr(rn_bmp_cmd_c::bmp_get_driver_name_c() as my_c_str).to_str() {
            Ok(x) => x,
            _ => "",
        }
    }
}
/*
 *
 */
pub fn bmp_mem_write(address: u32, data: &[u8]) -> bool {
    unsafe {
        let ptr: *const u8 = data.as_ptr();
        rn_bmp_cmd_c::bmp_mem_write_c(address, data.len() as u32, ptr)
    }
}

pub fn bmp_write_mem32(address: u32, data: &[u32]) -> bool {
    unsafe {
        // mem_read_c returns flase if ok (WTF)
        rn_bmp_cmd_c::bmp_mem_write_c(address, (data.len() as u32) * 4, data.as_ptr() as *const u8)
    }
}
/*
 *
 */
pub fn bmp_cpuid() -> u32 {
    unsafe { rn_bmp_cmd_c::bmp_get_cpuid_c() }
}
pub fn bmp_flash_complete() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_flash_complete_c() }
}

pub fn bmp_crc32(address: u32, length: u32) -> Option<u32> {
    unsafe {
        let mut crc: u32 = 0;
        let crc_ptr: *mut u32 = &mut crc;
        if rn_bmp_cmd_c::bmp_crc32_c(address, length, crc_ptr) {
            return Some(crc);
        }
        None
    }
}

pub fn bmp_read_mem_ptr(address: u32, size: u32, data: *mut u8) -> bool {
    unsafe {
        // mem_read_c returns flase if ok (WTF)
        rn_bmp_cmd_c::bmp_mem_read_c(address, size, data)
    }
}
pub fn bmp_read_mem(address: u32, data: &mut [u8]) -> bool {
    unsafe {
        // mem_read_c returns flase if ok (WTF)
        rn_bmp_cmd_c::bmp_mem_read_c(address, data.len() as u32, data.as_mut_ptr())
    }
}
pub fn bmp_read_mem32(address: u32, data: &mut [u32]) -> bool {
    unsafe {
        // mem_read_c returns flase if ok (WTF)
        rn_bmp_cmd_c::bmp_mem_read_c(
            address,
            (data.len() as u32) * 4,
            data.as_mut_ptr() as *mut u8,
        )
    }
}
pub fn bmp_reset_target() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_reset_target_c() }
}
/*
 * This adds wait state to the SWD signals i.e. slows it down
 */
pub fn bmp_set_wait_state(ws: u32) {
    unsafe {
        rn_bmp_cmd_c::bmp_set_wait_state_c(ws);
    }
}
/*
 *
 */
pub fn bmp_get_wait_state() -> u32 {
    unsafe { rn_bmp_cmd_c::bmp_get_wait_state_c() }
}
/*
 *
 */
pub fn bmp_set_frequency(fq: u32) {
    unsafe {
        rn_bmp_cmd_c::bmp_set_frequency_c(fq);
    }
}
/*
 *
 */
pub fn bmp_get_frequency() -> u32 {
    unsafe { rn_bmp_cmd_c::bmp_get_frequency_c() }
}

/*

typedef enum target_breakwatch {
    TARGET_BREAK_SOFT 0,
    TARGET_BREAK_HARD 1,
    TARGET_WATCH_WRITE 2,
    TARGET_WATCH_READ 3,
    TARGET_WATCH_ACCESS 4,
} target_breakwatch_e;

 */

pub fn bmp_add_breakpoint(btype: u32, adr: u32, len: u32) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_add_breakpoint_c(btype, adr, len) }
}
pub fn bmp_remove_breakpoint(btype: u32, adr: u32, len: u32) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_remove_breakpoint_c(btype, adr, len) }
}

pub fn bmp_target_halt() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_target_halt_c() }
}

// resume go or step by step
pub fn bmp_halt_resume(step: bool) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_target_halt_resume_c(step) }
}

pub fn bmp_poll() -> HaltState {
    unsafe {
        let mut wp: u32 = 0;
        let wp_ptr: *mut u32 = &mut wp;
        let r: u32 = rn_bmp_cmd_c::bmp_poll_target_c(wp_ptr);
        match r {
            0 => HaltState::Running,
            1 => HaltState::Error,
            2 => HaltState::Request,
            3 => HaltState::Stepping,
            4 => HaltState::Breakpoint,
            5 => HaltState::Watchpoint(wp),
            6 => HaltState::Fault,
            _ => panic!("wrong halt reason"),
        }
    }
}
//----------------------------------
pub fn rpc_init_swd() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_rpc_init_swd_c() }
}
//
pub fn swindleRedirectLog(onoff: bool) {
    unsafe {
        rn_bmp_cmd_c::swindleRedirectLog_c(onoff);
    }
}
/*
----------------- platform --------------
 */

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
pub fn dummyFun() -> *const u8 {
    let s: &'static str = "aaa";
    let p: *const u8 = s.as_ptr();
    p
}
/*
 *
 */
pub fn bmplog(s: &str) {
    unsafe {
        rn_bmp_cmd_c::Logger2(s.len() as i32, s.as_ptr());
    }
}
/*
 *
 */
pub fn bmp_adiv5_ap_read(device_index: u32, ap_selection: u32, address: u32) -> u32 {
    unsafe { rn_bmp_cmd_c::bmp_adiv5_ap_read_c(device_index, ap_selection, address) }
}
/*
 *
 */
pub fn bmp_clear_dp_fault() {
    unsafe {
        rn_bmp_cmd_c::bmp_clear_dp_fault_c();
    }
}
/*
 *
 */
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
/*
 *
 */
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
/*
 *
 */
pub fn bmp_adiv5_ap_write(device_index: u32, ap_selection: u32, address: u32, value: u32) {
    unsafe {
        rn_bmp_cmd_c::bmp_adiv5_ap_write_c(device_index, ap_selection, address, value);
    }
}
/*
 *
 */

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
/*
 *
 */
pub fn bmp_supported_boards() -> &'static str {
    unsafe {
        let boards = rn_bmp_cmd_c::list_enabled_boards();

        let output = CStr::from_ptr(boards as my_c_str).to_str();
        if let Ok(x) = output {
            return x;
        }
    }
    "--error--"
}
/*
 *
 */
pub fn bmp_get_version() -> &'static str {
    unsafe {
        match CStr::from_ptr(rn_bmp_cmd_c::bmp_get_version_string() as my_c_str).to_str() {
            Ok(x) => x,
            _ => "??",
        }
    }
}
/*
 *
 */
pub fn bmp_mon(input_as_string: &str) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_mon_c(input_as_string.as_ptr()) }
}
/*
 *
 */
pub fn free_heap() -> u32 {
    unsafe { rn_bmp_cmd_c::free_heap_c() }
}
/*
 *
 */
pub fn min_free_heap() -> u32 {
    unsafe { rn_bmp_cmd_c::min_free_heap_c() }
}

pub fn get_heap_stats() -> (u32, u32) {
    (min_free_heap(), free_heap())
}

/*
 pub fn bmp_platform_target_voltage() ->
 {
    unsafe {
        rn_bmp_cmd_c::platform_target_voltage()
    }
 }
*/
/*
 *
 */
pub fn rv_dm_start() {
    unsafe {
        rn_bmp_cmd_c::rv_dm_start_c();
    }
}

/*
 *
 */
pub fn bmp_rv_reset() -> bool {
    unsafe {
        let status: bool = rn_bmp_cmd_c::bmp_rv_dm_reset_c();
        status
    }
}

/*
 *
 */

pub fn bmp_rv_read(adr: u8) -> (bool, u32) {
    unsafe {
        let mut ret: u32 = 0;
        let ret_ptr: *mut u32 = &mut ret;

        let status: bool = rn_bmp_cmd_c::bmp_rv_dm_read_c(adr, ret_ptr);
        (status, ret)
    }
}
/*
 *
 */
pub fn bmp_rv_write(adr: u8, data: u32) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_rv_dm_write_c(adr, data) }
}

pub fn bmp_rvswdp_probe(id: &mut u32) -> bool {
    let id_ptr: *mut u32 = id;
    unsafe { rn_bmp_cmd_c::bmp_rv_rvswd_probe_c(id_ptr) }
}
//
// ln_adiv wrapper
//
pub fn bmp_adiv5_swd_write_no_check(addr: u16, data: u32) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_adiv5_swd_write_no_check_c(addr, data) }
}
pub fn bmp_adiv5_swd_read_no_check(addr: u16) -> u32 {
    unsafe { rn_bmp_cmd_c::bmp_adiv5_swd_read_no_check_c(addr) }
}
pub fn bmp_raw_swd_write(tick: u32, value: u32) {
    unsafe { rn_bmp_cmd_c::bmp_raw_swd_write_c(tick, value) }
}
pub fn bmp_adiv5_swd_raw_access(rnw: u8, addr: u16, value: u32, fault: *mut u32) -> u32 {
    unsafe { rn_bmp_cmd_c::bmp_adiv5_swd_raw_access_c(rnw, addr, value, fault) }
}
pub fn bmp_is_riscv() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_is_riscv_c() }
}
//
pub fn bmp_ch32v3xx_write_user_option_byte(memory_conf: u8) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_ch32v3xx_write_user_option_byte_c(memory_conf) }
}
//
pub fn bmp_ch32v3xx_read_user_option_byte() -> u8 {
    unsafe { rn_bmp_cmd_c::bmp_ch32v3xx_read_user_option_byte_c() }
}
//
pub fn bmp_custom_crc32(address: u32, size_in_bytes: u32) -> (bool, u32) {
    let mut crc: u32 = 0;
    unsafe {
        let r = rn_bmp_cmd_c::bmp_custom_crc32_c(address, size_in_bytes, &mut crc);
        (r, crc)
    }
}
/*
 *
 */
pub fn bmp_raise_exception() {
    unsafe {
        rn_bmp_cmd_c::bmp_raise_exception_c();
    }
}
/*
 *
 */
pub fn bmp_try() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_try_c() }
}
/*
 *
 */
pub fn bmp_catch() -> i32 {
    unsafe { rn_bmp_cmd_c::bmp_catch_c() }
}
/*
*
*/
pub fn bmp_enable_reset_pin(enabled: bool) {
    unsafe {
        rn_bmp_cmd_c::bmp_enable_reset_pin_c(enabled);
    }
}
pub fn bmp_watchpoint_breakpoint_count() -> (u32, u32) {
    unsafe {
        let mut brk: u32 = 0;
        let mut wtch: u32 = 0;
        rn_bmp_cmd_c::bmp_breakpoint_watchpoint_count_c(&mut brk, &mut wtch);
        (brk, wtch)
    }
}
pub fn bmp_has_hw_breakpoint() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_has_hw_breakpoint_c() }
}
/*
 */
// EOF
