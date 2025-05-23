#![allow(unsafe_op_in_unsafe_fn)]
/* automatically generated by rust-bindgen 0.71.1 */

pub const _ELIX_LEVEL: u32 = 4;
pub const _NEWLIB_VERSION: &[u8; 6] = b"4.3.0\0";
pub const _PICOLIBC_CTYPE_SMALL: u32 = 0;
pub const _PICOLIBC_MINOR__: u32 = 8;
pub const _PICOLIBC_VERSION: &[u8; 6] = b"1.8.9\0";
pub const _PICOLIBC__: u32 = 1;
pub const __NEWLIB_MINOR__: u32 = 3;
pub const __NEWLIB_PATCHLEVEL__: u32 = 0;
pub const __NEWLIB__: u32 = 4;
pub const __PICOLIBC_MINOR__: u32 = 8;
pub const __PICOLIBC_PATCHLEVEL__: u32 = 9;
pub const __PICOLIBC_VERSION__: &[u8; 6] = b"1.8.9\0";
pub const __PICOLIBC__: u32 = 1;
pub const _ATFILE_SOURCE: u32 = 1;
pub const _DEFAULT_SOURCE: u32 = 1;
pub const _ISOC99_SOURCE: u32 = 1;
pub const _ISOC11_SOURCE: u32 = 1;
pub const _ISOC23_SOURCE: u32 = 1;
pub const _POSIX_SOURCE: u32 = 1;
pub const _POSIX_C_SOURCE: u32 = 200809;
pub const _XOPEN_SOURCE: u32 = 700;
pub const _XOPEN_SOURCE_EXTENDED: u32 = 1;
pub const _LARGEFILE64_SOURCE: u32 = 1;
pub const __ATFILE_VISIBLE: u32 = 1;
pub const __BSD_VISIBLE: u32 = 1;
pub const __GNU_VISIBLE: u32 = 1;
pub const __ZEPHYR_VISIBLE: u32 = 0;
pub const __ISO_C_VISIBLE: u32 = 2023;
pub const __LARGEFILE_VISIBLE: u32 = 1;
pub const __LARGEFILE64_VISIBLE: u32 = 1;
pub const __MISC_VISIBLE: u32 = 1;
pub const __POSIX_VISIBLE: u32 = 202405;
pub const __SVID_VISIBLE: u32 = 1;
pub const __XSI_VISIBLE: u32 = 700;
pub const __SSP_FORTIFY_LEVEL: u32 = 0;
pub const __OBSOLETE_MATH_DEFAULT: u32 = 1;
pub const __OBSOLETE_MATH: u32 = 1;
pub const __OBSOLETE_MATH_FLOAT: u32 = 1;
pub const __OBSOLETE_MATH_DOUBLE: u32 = 1;
pub const __RAND_MAX: u32 = 2147483647;
pub const __have_longlong64: u32 = 1;
pub const __have_long32: u32 = 1;
pub const __SCHAR_WIDTH__: u32 = 8;
pub const __LONG_LONG_WIDTH__: u32 = 64;
pub const ___int8_t_defined: u32 = 1;
pub const ___int16_t_defined: u32 = 1;
pub const ___int32_t_defined: u32 = 1;
pub const ___int64_t_defined: u32 = 1;
pub const ___int_least8_t_defined: u32 = 1;
pub const ___int_least16_t_defined: u32 = 1;
pub const ___int_least32_t_defined: u32 = 1;
pub const ___int_least64_t_defined: u32 = 1;
pub const __GNUCLIKE_ASM: u32 = 3;
pub const __GNUCLIKE___TYPEOF: u32 = 1;
pub const __GNUCLIKE___SECTION: u32 = 1;
pub const __GNUCLIKE_PRAGMA_DIAGNOSTIC: u32 = 1;
pub const __GNUCLIKE_CTOR_SECTION_HANDLING: u32 = 1;
pub const __GNUCLIKE_BUILTIN_CONSTANT_P: u32 = 1;
pub const __GNUCLIKE_BUILTIN_VARARGS: u32 = 1;
pub const __GNUCLIKE_BUILTIN_STDARG: u32 = 1;
pub const __GNUCLIKE_BUILTIN_VAALIST: u32 = 1;
pub const __GNUC_VA_LIST_COMPATIBILITY: u32 = 1;
pub const __GNUCLIKE_BUILTIN_NEXT_ARG: u32 = 1;
pub const __GNUCLIKE_BUILTIN_MEMCPY: u32 = 1;
pub const __CC_SUPPORTS_INLINE: u32 = 1;
pub const __CC_SUPPORTS___INLINE: u32 = 1;
pub const __CC_SUPPORTS___INLINE__: u32 = 1;
pub const __CC_SUPPORTS___FUNC__: u32 = 1;
pub const __CC_SUPPORTS_WARNING: u32 = 1;
pub const __CC_SUPPORTS_VARADIC_XXX: u32 = 1;
pub const __CC_SUPPORTS_DYNAMIC_ARRAY_INIT: u32 = 1;
pub const __int20: u32 = 2;
pub const __int20__: u32 = 2;
pub const __INT8: &[u8; 3] = b"hh\0";
pub const __INT16: &[u8; 2] = b"h\0";
pub const __INT64: &[u8; 3] = b"ll\0";
pub const __FAST8: &[u8; 3] = b"hh\0";
pub const __FAST16: &[u8; 2] = b"h\0";
pub const __FAST64: &[u8; 3] = b"ll\0";
pub const __LEAST8: &[u8; 3] = b"hh\0";
pub const __LEAST16: &[u8; 2] = b"h\0";
pub const __LEAST64: &[u8; 3] = b"ll\0";
pub const __int8_t_defined: u32 = 1;
pub const __int16_t_defined: u32 = 1;
pub const __int32_t_defined: u32 = 1;
pub const __int64_t_defined: u32 = 1;
pub const __int_least8_t_defined: u32 = 1;
pub const __int_least16_t_defined: u32 = 1;
pub const __int_least32_t_defined: u32 = 1;
pub const __int_least64_t_defined: u32 = 1;
pub const __int_fast8_t_defined: u32 = 1;
pub const __int_fast16_t_defined: u32 = 1;
pub const __int_fast32_t_defined: u32 = 1;
pub const __int_fast64_t_defined: u32 = 1;
pub const WINT_MIN: u32 = 0;
pub type __int8_t = cty::c_schar;
pub type __uint8_t = cty::c_uchar;
pub type __int16_t = cty::c_short;
pub type __uint16_t = cty::c_ushort;
pub type __int32_t = cty::c_int;
pub type __uint32_t = cty::c_uint;
pub type __int64_t = cty::c_longlong;
pub type __uint64_t = cty::c_ulonglong;
pub type __int_least8_t = cty::c_schar;
pub type __uint_least8_t = cty::c_uchar;
pub type __int_least16_t = cty::c_short;
pub type __uint_least16_t = cty::c_ushort;
pub type __int_least32_t = cty::c_int;
pub type __uint_least32_t = cty::c_uint;
pub type __int_least64_t = cty::c_longlong;
pub type __uint_least64_t = cty::c_ulonglong;
pub type __intmax_t = cty::c_longlong;
pub type __uintmax_t = cty::c_ulonglong;
pub type __intptr_t = cty::c_int;
pub type __uintptr_t = cty::c_uint;
pub type intmax_t = __intmax_t;
pub type uintmax_t = __uintmax_t;
pub type int_least8_t = __int_least8_t;
pub type uint_least8_t = __uint_least8_t;
pub type int_least16_t = __int_least16_t;
pub type uint_least16_t = __uint_least16_t;
pub type int_least32_t = __int_least32_t;
pub type uint_least32_t = __uint_least32_t;
pub type int_least64_t = __int_least64_t;
pub type uint_least64_t = __uint_least64_t;
pub type int_fast8_t = cty::c_schar;
pub type uint_fast8_t = cty::c_uchar;
pub type int_fast16_t = cty::c_short;
pub type uint_fast16_t = cty::c_ushort;
pub type int_fast32_t = cty::c_int;
pub type uint_fast32_t = cty::c_uint;
pub type int_fast64_t = cty::c_longlong;
pub type uint_fast64_t = cty::c_ulonglong;
pub type target_s = cty::c_void;
pub type bool_ = cty::c_int;
unsafe extern "C" {
    pub fn bmp_custom_crc32_c(
        adr: cty::c_uint,
        size_in_bytes: cty::c_uint,
        crc: *mut cty::c_uint,
    ) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_get_driver_name_c() -> *const cty::c_uchar;
}
unsafe extern "C" {
    pub fn bmp_ch32v3xx_write_user_option_byte_c(memory_conf: u8) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_ch32v3xx_read_user_option_byte_c() -> u8;
}
unsafe extern "C" {
    pub fn bmp_set_wait_state_c(ws: cty::c_uint);
}
unsafe extern "C" {
    pub fn bmp_get_wait_state_c() -> cty::c_uint;
}
unsafe extern "C" {
    pub fn bmp_set_frequency_c(fs: cty::c_uint);
}
unsafe extern "C" {
    pub fn bmp_get_frequency_c() -> cty::c_uint;
}
unsafe extern "C" {
    pub fn cmd_swd_scan(
        t: *const target_s,
        argc: cty::c_int,
        argv: *mut *const cty::c_uchar,
    ) -> bool_;
}
unsafe extern "C" {
    pub fn cmd_rvswd_scan(
        t: *const target_s,
        argc: cty::c_int,
        argv: *mut *const cty::c_uchar,
    ) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_attach_c(target: cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_detach_c(target: cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_attached_c() -> bool_;
}
unsafe extern "C" {
    pub fn bmp_is_riscv_c() -> bool_;
}
unsafe extern "C" {
    pub fn bmp_map_count_c(kind: cty::c_uint) -> cty::c_int;
}
unsafe extern "C" {
    pub fn bmp_map_get_c(
        arg1: cty::c_uint,
        arg2: cty::c_uint,
        start: *mut cty::c_uint,
        size: *mut cty::c_uint,
        blockSize: *mut cty::c_uint,
    ) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_registers_count_c() -> cty::c_uint;
}
unsafe extern "C" {
    pub fn bmp_read_register_c(reg: cty::c_uint, val: *mut cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_read_registers_c(val: *mut cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_read_all_registers_c(regs: *mut cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_write_all_registers_c(regs: *const cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_target_description_c() -> *const cty::c_uchar;
}
unsafe extern "C" {
    pub fn bmp_target_description_clear_c();
}
unsafe extern "C" {
    pub fn bmp_write_reg_c(reg: cty::c_uint, value: cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_read_reg_c(reg: cty::c_uint, value: *mut cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_get_cpuid_c() -> cty::c_uint;
}
unsafe extern "C" {
    pub fn bmp_flash_erase_c(addr: cty::c_uint, length: cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_flash_write_c(addr: cty::c_uint, length: cty::c_uint, data: *const u8) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_flash_complete_c() -> bool_;
}
unsafe extern "C" {
    pub fn bmp_get_target_voltage_c() -> f32;
}
unsafe extern "C" {
    pub fn bmp_mem_read_c(addr: cty::c_uint, length: cty::c_uint, data: *mut u8) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_crc32_c(address: cty::c_uint, length: cty::c_uint, crc: *mut cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_reset_target_c() -> bool_;
}
unsafe extern "C" {
    pub fn rv_dm_start_c();
}
unsafe extern "C" {
    pub fn bmp_add_breakpoint_c(
        type_: cty::c_uint,
        address: cty::c_uint,
        len: cty::c_uint,
    ) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_remove_breakpoint_c(
        type_: cty::c_uint,
        address: cty::c_uint,
        len: cty::c_uint,
    ) -> bool_;
}
unsafe extern "C" {
    pub fn riscv_list_csr(
        start: cty::c_uint,
        max_size: cty::c_uint,
        csr: *mut cty::c_uint,
    ) -> cty::c_uint;
}
unsafe extern "C" {
    pub fn bmp_target_halt_resume_c(step: bool_) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_target_halt_c() -> bool_;
}
unsafe extern "C" {
    pub fn bmp_poll_target_c(watchpoint: *mut cty::c_uint) -> cty::c_uint;
}
unsafe extern "C" {
    pub fn bmp_rpc_init_swd_c() -> bool_;
}
unsafe extern "C" {
    pub fn bmp_rpc_swd_in_c(value: *mut cty::c_uint, nb_bits: cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_rpc_swd_in_par_c(
        value: *mut cty::c_uint,
        par: *mut bool_,
        nb_bits: cty::c_uint,
    ) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_rpc_swd_out_c(value: cty::c_uint, nb_bits: cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_rpc_swd_out_par_c(value: cty::c_uint, nb_bits: cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_mem_write_c(address: cty::c_uint, len: cty::c_uint, data: *const u8) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_adiv5_full_dp_read_c(
        device_index: cty::c_uint,
        ap_selection: cty::c_uint,
        address: u16,
        err: *mut i32,
        value: *mut cty::c_uint,
    ) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_adiv5_full_dp_low_level_c(
        device_index: cty::c_uint,
        ap_selection: cty::c_uint,
        address: u16,
        value: cty::c_uint,
        err: *mut i32,
        outvalue: *mut cty::c_uint,
    ) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_adiv5_ap_read_c(
        device_index: cty::c_uint,
        ap_selection: cty::c_uint,
        address: cty::c_uint,
    ) -> cty::c_uint;
}
unsafe extern "C" {
    pub fn bmp_adiv5_ap_write_c(
        device_index: cty::c_uint,
        ap_selection: cty::c_uint,
        address: cty::c_uint,
        value: cty::c_uint,
    );
}
unsafe extern "C" {
    pub fn bmp_adiv5_mem_read_c(
        device_index: cty::c_uint,
        ap_selection: cty::c_uint,
        csw: cty::c_uint,
        address: cty::c_uint,
        buffer: *mut u8,
        len: cty::c_uint,
    ) -> i32;
}
unsafe extern "C" {
    pub fn bmp_adiv5_mem_write_c(
        device_index: cty::c_uint,
        ap_selection: cty::c_uint,
        csw: cty::c_uint,
        address: cty::c_uint,
        align: cty::c_uint,
        buffer: *const u8,
        len: cty::c_uint,
    ) -> i32;
}
unsafe extern "C" {
    pub fn swindleRedirectLog_c(onoff: bool_);
}
unsafe extern "C" {
    pub fn platform_nrst_set_val(assert: bool_);
}
unsafe extern "C" {
    pub fn platform_nrst_get_val() -> bool_;
}
unsafe extern "C" {
    pub fn platform_target_voltage() -> *const cty::c_uchar;
}
unsafe extern "C" {
    pub fn platform_target_clk_output_enable(enable: bool_);
}
unsafe extern "C" {
    pub fn Logger2(n: cty::c_int, fmt: *const cty::c_uchar);
}
unsafe extern "C" {
    pub fn list_enabled_boards() -> *const cty::c_uchar;
}
unsafe extern "C" {
    pub fn bmp_pin_set(pin: u8, value: u8);
}
unsafe extern "C" {
    pub fn bmp_pin_get(pin: u8) -> u8;
}
unsafe extern "C" {
    pub fn bmp_pin_direction(pin: u8, output: u8);
}
unsafe extern "C" {
    pub fn bmp_test();
}
unsafe extern "C" {
    pub fn bmp_get_version_string() -> *const cty::c_uchar;
}
unsafe extern "C" {
    pub fn bmp_mon_c(str_: *const cty::c_uchar) -> bool_;
}
unsafe extern "C" {
    pub fn free_heap_c() -> cty::c_uint;
}
unsafe extern "C" {
    pub fn min_free_heap_c() -> cty::c_uint;
}
unsafe extern "C" {
    pub fn bmp_rv_dm_read_c(adr: u8, value: *mut cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_rv_dm_write_c(adr: u8, value: cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_rv_dm_reset_c() -> bool_;
}
unsafe extern "C" {
    pub fn bmp_rv_rvswd_probe_c(id: *mut cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn platform_buffer_read(data: *mut u8, maxsize: cty::c_int) -> cty::c_int;
}
unsafe extern "C" {
    pub fn platform_buffer_write(data: *const u8, size: cty::c_int) -> cty::c_int;
}
unsafe extern "C" {
    pub fn platform_buffer_write_buffered(data: *const u8, size: cty::c_int) -> cty::c_int;
}
unsafe extern "C" {
    pub fn platform_write_flush();
}
unsafe extern "C" {
    pub fn bmp_clear_dp_fault_c();
}
unsafe extern "C" {
    pub fn bmp_adiv5_swd_write_no_check_c(addr: u16, data: cty::c_uint) -> bool_;
}
unsafe extern "C" {
    pub fn bmp_adiv5_swd_read_no_check_c(addr: u16) -> cty::c_uint;
}
unsafe extern "C" {
    pub fn bmp_raw_swd_write_c(tick: cty::c_uint, value: cty::c_uint);
}
unsafe extern "C" {
    pub fn bmp_adiv5_swd_raw_access_c(
        rnw: u8,
        addr: u16,
        value: cty::c_uint,
        fault: *mut cty::c_uint,
    ) -> cty::c_uint;
}
pub const rttField_ENABLED: rttField = 0;
pub const rttField_ADDRESS: rttField = 1;
pub const rttField_POLLING: rttField = 2;
pub type rttField = cty::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct rttInfo {
    pub enabled: cty::c_uint,
    pub min_address: cty::c_uint,
    pub max_address: cty::c_uint,
    pub min_poll_ms: cty::c_uint,
    pub max_poll_ms: cty::c_uint,
    pub max_poll_error: cty::c_uint,
    pub found: cty::c_uint,
    pub cb_address: cty::c_uint,
}
unsafe extern "C" {
    pub fn bmp_rtt_get_info_c(info: *mut rttInfo);
}
unsafe extern "C" {
    pub fn bmp_rtt_set_info_c(field: rttField, info: *const rttInfo);
}
unsafe extern "C" {
    pub fn bmp_raise_exception_c();
}
unsafe extern "C" {
    pub fn bmp_try_c() -> bool_;
}
unsafe extern "C" {
    pub fn bmp_catch_c() -> cty::c_int;
}
