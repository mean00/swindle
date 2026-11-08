//! C-ABI translation layer adapting legacy BMP target calls to/from Rust.
//!
//! Exclusively used by legacy C drivers and C TAP scanners. Pure Rust drivers
//! do not reference this module.

use core::ffi::{c_char, c_void};
use super::target;

/// Representation of a target instance matching the legacy C ABI layout.
#[repr(C)]
pub struct TargetC {
    pub driver: *const c_char,
    pub core: *const c_char,
    pub cpuid: u32,
    pub designer_code: u16,
    pub part_id: u16,
    pub target_options: u32,
    pub regs_size: usize,
    pub attached: bool,
    pub no_hw_breakpoint: bool,
    pub is_cortexm: bool,
    pub flash_mode: bool,
    pub enter_flash_mode_stub: bool,
    pub priv_state: *mut c_void,
    pub priv_free: Option<unsafe extern "C" fn(priv_: *mut c_void)>,
    pub attach: Option<unsafe extern "C" fn(t: *mut TargetC) -> bool>,
    pub detach: Option<unsafe extern "C" fn(t: *mut TargetC)>,
    pub mass_erase: Option<unsafe extern "C" fn(target: *mut TargetC, context: *mut c_void) -> bool>,
    pub next: *mut TargetC,
}

// ---------------------------------------------------------------------------
// Exported C-ABI Functions (for legacy C target drivers and TAP scanners)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_list_free() {
    unsafe {
        target::target_list_free();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_add_ram32(_target: *mut TargetC, _start: u32, _len: u32) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_add_flash(_target: *mut TargetC, _flash: *mut c_void) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem_map_free(_target: *mut TargetC) {}
