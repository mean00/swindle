//! C-ABI translation layer adapting BMP target calls to/from idiomatic Rust.

use super::driver::{HaltReason, TargetDriver, TargetError};
use core::ffi::c_void;

/// Opaque handle to C `target_s` struct.
pub enum TargetC {}

/// Representation of C `target_ram_s`.
#[repr(C)]
pub struct TargetRamC {
    pub start: u32,
    pub length: usize,
    pub next: *mut TargetRamC,
}

unsafe extern "C" {
    fn target_alloc_c() -> *mut TargetC;
    fn target_free_c(t: *mut TargetC);
    fn target_detach_safe(t: *mut TargetC);
    fn target_ram_alloc_c() -> *mut TargetRamC;
    fn target_ram_free_c(r: *mut TargetRamC);
    fn target_flash_buf_free_c(flash: *mut c_void);
    fn target_priv_free_c(t: *mut TargetC);
    fn target_storage_free_c(t: *mut TargetC);
    fn target_commands_free_c(t: *mut TargetC);
    fn target_bw_list_free_c(t: *mut TargetC);
    fn target_get_tc_destroy_c(t: *mut TargetC);

    fn target_get_next_c(t: *mut TargetC) -> *mut TargetC;
    fn target_set_next_c(t: *mut TargetC, next: *mut TargetC);
    fn target_set_attached_c(t: *mut TargetC, att: bool);
    fn target_set_tc_c(t: *mut TargetC, tc: *mut c_void);
    fn target_call_attach_c(t: *mut TargetC) -> bool;
    fn target_call_detach_c(t: *mut TargetC);
    fn target_call_check_error_c(t: *mut TargetC) -> bool;
    fn target_call_mem_read_c(t: *mut TargetC, dest: *mut c_void, src: u64, len: usize);
    fn target_call_mem_write_c(t: *mut TargetC, dest: u64, src: *const c_void, len: usize);
    fn target_call_mem_read_nostop_c(t: *mut TargetC, dest: *mut c_void, src: u64, len: usize) -> bool;
    fn target_call_mem_write_nostop_c(t: *mut TargetC, dest: u64, src: *const c_void, len: usize) -> bool;
    fn target_needs_halt_for_mem_c(t: *mut TargetC) -> bool;
    fn target_call_reg_read_c(t: *mut TargetC, reg: u32, data: *mut c_void, max: usize) -> usize;
    fn target_call_reg_write_c(t: *mut TargetC, reg: u32, data: *const c_void, size: usize) -> usize;
    fn target_call_regs_read_c(t: *mut TargetC, data: *mut c_void) -> bool;
    fn target_call_regs_write_c(t: *mut TargetC, data: *const c_void) -> bool;
    fn target_get_regs_size_c(t: *mut TargetC) -> usize;
    fn target_call_reset_c(t: *mut TargetC);
    fn target_call_halt_request_c(t: *mut TargetC);
    fn target_call_halt_poll_c(t: *mut TargetC, watch: *mut u64) -> u32;
    fn target_call_halt_resume_c(t: *mut TargetC, step: bool);
    fn target_get_ram_c(t: *mut TargetC) -> *mut TargetRamC;
    fn target_set_ram_c(t: *mut TargetC, ram: *mut TargetRamC);
    fn target_get_flash_c(t: *mut TargetC) -> *mut c_void;
    fn target_set_flash_c(t: *mut TargetC, flash: *mut c_void);
    fn target_flash_get_next_c(flash: *mut c_void) -> *mut c_void;
    fn target_flash_setup_c(t: *mut TargetC, flash: *mut c_void);
    fn target_add_default_commands_c(t: *mut TargetC);
    fn platform_target_clk_output_enable(enable: bool);
    pub fn target_get_attached_c(t: *mut TargetC) -> bool;
    pub fn target_get_driver_name_c(t: *mut TargetC) -> *const core::ffi::c_char;
    pub fn target_get_cpuid_c(t: *mut TargetC) -> u32;
    pub fn target_breakwatch_set(target: *mut TargetC, btype: u32, addr: u32, len: usize) -> i32;
    pub fn target_breakwatch_clear(target: *mut TargetC, btype: u32, addr: u32, len: usize) -> i32;
    pub fn target_breakpoint_watchpoint_count(target: *mut c_void, brk: *mut u32, wtch: *mut u32);
    pub fn target_has_hw_breakpoint(target: *mut TargetC) -> bool;
    pub fn target_get_options_c(t: *mut TargetC) -> u32;
    pub fn target_get_core_c(t: *mut TargetC) -> *const core::ffi::c_char;
    pub fn target_has_mw_helpers_c(t: *mut TargetC) -> bool;
    pub fn target_mw_page_size_c(t: *mut TargetC) -> u32;
    pub fn target_overwrite_flash_c(t: *mut TargetC, address: u32, data: *const u8, len: u32) -> bool;
    pub static mut cur_target: *mut TargetC;
}

pub fn get_cur_target() -> *mut TargetC {
    unsafe { cur_target }
}

pub fn current_driver() -> Option<CTargetDriver> {
    let t = unsafe { cur_target };
    if t.is_null() {
        None
    } else {
        Some(unsafe { CTargetDriver::new(t) })
    }
}

pub fn target_ram_count(target: *mut TargetC) -> usize {
    let mut count = 0;
    unsafe {
        let mut cur = target_get_ram_c(target);
        while !cur.is_null() {
            count += 1;
            cur = (*cur).next;
        }
    }
    count
}

pub fn target_ram_get(target: *mut TargetC, index: usize) -> Option<(u32, u32)> {
    unsafe {
        let mut cur = target_get_ram_c(target);
        let mut idx = 0;
        while !cur.is_null() {
            if idx == index {
                return Some(((*cur).start, (*cur).length as u32));
            }
            idx += 1;
            cur = (*cur).next;
        }
    }
    None
}

pub fn target_flash_count(target: *mut TargetC) -> usize {
    let mut count = 0;
    unsafe {
        let mut cur = target_get_flash_c(target);
        while !cur.is_null() {
            count += 1;
            cur = target_flash_get_next_c(cur);
        }
    }
    count
}

pub fn target_flash_get(target: *mut TargetC, index: usize) -> Option<(u32, u32, u32)> {
    unsafe {
        let mut cur = target_get_flash_c(target);
        let mut idx = 0;
        while !cur.is_null() {
            if idx == index {
                let f = cur as *const crate::flash::c_adapter::TargetFlash;
                return Some(((*f).start, (*f).length as u32, (*f).blocksize as u32));
            }
            idx += 1;
            cur = target_flash_get_next_c(cur);
        }
    }
    None
}

/// Global target list pointer accessed by C subsystems (e.g. adiv5_swd.c, adi.c).
#[unsafe(no_mangle)]
pub static mut target_list: *mut TargetC = core::ptr::null_mut();

/// Adapter implementing `TargetDriver` for an active `target_s` instance.
pub struct CTargetDriver {
    target: *mut TargetC,
}

impl CTargetDriver {
    /// Wrap a C target pointer.
    ///
    /// # Safety
    /// `target` must be a valid, allocated `target_s` pointer.
    pub unsafe fn new(target: *mut TargetC) -> Self {
        Self { target }
    }

    pub fn raw(&self) -> *mut TargetC {
        self.target
    }
}

impl TargetDriver for CTargetDriver {
    fn mem_read(&self, dest: &mut [u8], addr: u32) -> Result<(), TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        unsafe {
            target_call_mem_read_c(self.target, dest.as_mut_ptr() as *mut c_void, addr as u64, dest.len());
            if target_call_check_error_c(self.target) {
                Err(TargetError::AccessFault)
            } else {
                Ok(())
            }
        }
    }

    fn mem_write(&self, addr: u32, src: &[u8]) -> Result<(), TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        unsafe {
            target_call_mem_write_c(self.target, addr as u64, src.as_ptr() as *const c_void, src.len());
            if target_call_check_error_c(self.target) {
                Err(TargetError::AccessFault)
            } else {
                Ok(())
            }
        }
    }

    fn mem_read_nostop(&self, dest: &mut [u8], addr: u32) -> Result<bool, TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        unsafe {
            Ok(target_call_mem_read_nostop_c(self.target, dest.as_mut_ptr() as *mut c_void, addr as u64, dest.len()))
        }
    }

    fn mem_write_nostop(&self, addr: u32, src: &[u8]) -> Result<bool, TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        unsafe {
            Ok(target_call_mem_write_nostop_c(self.target, addr as u64, src.as_ptr() as *const c_void, src.len()))
        }
    }

    fn needs_halt_for_mem(&self) -> bool {
        if self.target.is_null() {
            return true;
        }
        unsafe { target_needs_halt_for_mem_c(self.target) }
    }

    fn reg_read(&self, reg: u32, buf: &mut [u8]) -> Result<usize, TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        unsafe {
            let n = target_call_reg_read_c(self.target, reg, buf.as_mut_ptr() as *mut c_void, buf.len());
            Ok(n)
        }
    }

    fn reg_write(&self, reg: u32, data: &[u8]) -> Result<usize, TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        unsafe {
            let n = target_call_reg_write_c(self.target, reg, data.as_ptr() as *const c_void, data.len());
            Ok(n)
        }
    }

    fn regs_read(&self, buf: &mut [u8]) -> Result<(), TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        unsafe {
            if !target_call_regs_read_c(self.target, buf.as_mut_ptr() as *mut c_void) {
                let total = target_get_regs_size_c(self.target);
                let mut offset = 0;
                let mut reg_idx = 0;
                while offset < total && offset < buf.len() {
                    let chunk = target_call_reg_read_c(
                        self.target,
                        reg_idx,
                        buf[offset..].as_mut_ptr() as *mut c_void,
                        total - offset,
                    );
                    if chunk == 0 {
                        break;
                    }
                    offset += chunk;
                    reg_idx += 1;
                }
            }
            Ok(())
        }
    }

    fn regs_write(&self, data: &[u8]) -> Result<(), TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        unsafe {
            if !target_call_regs_write_c(self.target, data.as_ptr() as *const c_void) {
                let total = target_get_regs_size_c(self.target);
                let mut offset = 0;
                let mut reg_idx = 0;
                while offset < total && offset < data.len() {
                    let chunk = target_call_reg_write_c(
                        self.target,
                        reg_idx,
                        data[offset..].as_ptr() as *const c_void,
                        total - offset,
                    );
                    if chunk == 0 {
                        break;
                    }
                    offset += chunk;
                    reg_idx += 1;
                }
            }
            Ok(())
        }
    }

    fn regs_size(&self) -> usize {
        if self.target.is_null() {
            return 0;
        }
        unsafe { target_get_regs_size_c(self.target) }
    }

    fn reset(&self) {
        if !self.target.is_null() {
            unsafe { target_call_reset_c(self.target) }
        }
    }

    fn halt_request(&self) {
        if !self.target.is_null() {
            unsafe { target_call_halt_request_c(self.target) }
        }
    }

    fn halt_poll(&self) -> (HaltReason, u32) {
        if self.target.is_null() {
            return (HaltReason::Running, 0);
        }
        unsafe {
            let mut watch: u64 = 0;
            let reason_raw = target_call_halt_poll_c(self.target, &mut watch);
            (HaltReason::from(reason_raw), watch as u32)
        }
    }

    fn halt_resume(&self, step: bool) {
        if !self.target.is_null() {
            unsafe { target_call_halt_resume_c(self.target, step) }
        }
    }

    fn attach(&mut self) -> Result<(), TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        unsafe {
            if target_call_attach_c(self.target) {
                target_set_attached_c(self.target, true);
                Ok(())
            } else {
                Err(TargetError::AccessFault)
            }
        }
    }

    fn detach(&mut self) {
        if !self.target.is_null() {
            unsafe {
                target_call_detach_c(self.target);
                target_set_attached_c(self.target, false);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Exported C-ABI Functions (Replacing blackmagic/src/target/target.c)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_new() -> *mut TargetC {
    unsafe {
        let target = target_alloc_c();
        if target.is_null() {
            return core::ptr::null_mut();
        }

        if target_list.is_null() {
            target_list = target;
        } else {
            let mut cur = target_list;
            while !target_get_next_c(cur).is_null() {
                cur = target_get_next_c(cur);
            }
            target_set_next_c(cur, target);
        }

        target_add_default_commands_c(target);
        target
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_list_get_last() -> *mut TargetC {
    unsafe {
        if target_list.is_null() {
            return core::ptr::null_mut();
        }
        let mut cur = target_list;
        while !target_get_next_c(cur).is_null() {
            cur = target_get_next_c(cur);
        }
        cur
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_foreach(
    callback: Option<unsafe extern "C" fn(index: usize, target: *mut TargetC, context: *mut c_void)>,
    context: *mut c_void,
) -> usize {
    unsafe {
        let mut idx = 0;
        let mut cur = target_list;
        while !cur.is_null() {
            idx += 1;
            if let Some(cb) = callback {
                cb(idx, cur, context);
            }
            cur = target_get_next_c(cur);
        }
        idx
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_attach_n(n: usize, controller: *mut c_void) -> *mut TargetC {
    unsafe {
        let mut idx = 0;
        let mut cur = target_list;
        while !cur.is_null() {
            idx += 1;
            if idx == n {
                return target_attach(cur, controller);
            }
            cur = target_get_next_c(cur);
        }
        core::ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_attach(target: *mut TargetC, controller: *mut c_void) -> *mut TargetC {
    if target.is_null() {
        return core::ptr::null_mut();
    }

    unsafe {
        target_get_tc_destroy_c(target);
        target_set_tc_c(target, controller);
        platform_target_clk_output_enable(true);

        if !target_call_attach_c(target) {
            platform_target_clk_output_enable(false);
            return core::ptr::null_mut();
        }

        target_set_attached_c(target, true);
        target
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_detach(target: *mut TargetC) {
    if target.is_null() {
        return;
    }
    unsafe {
        target_call_detach_c(target);
        platform_target_clk_output_enable(false);
        target_set_attached_c(target, false);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_list_free() {
    unsafe {
        let mut cur = target_list;
        while !cur.is_null() {
            let next = target_get_next_c(cur);

            target_detach_safe(cur);
            target_get_tc_destroy_c(cur);
            target_priv_free_c(cur);
            target_commands_free_c(cur);
            target_storage_free_c(cur);
            target_mem_map_free(cur);
            target_bw_list_free_c(cur);
            target_free_c(cur);

            cur = next;
        }
        target_list = core::ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_add_ram32(target: *mut TargetC, start: u32, len: u32) {
    unsafe { target_add_ram64(target, start as u64, len as u64) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_add_ram64(target: *mut TargetC, start: u64, len: u64) {
    if target.is_null() {
        return;
    }
    unsafe {
        let ram = target_ram_alloc_c();
        if ram.is_null() {
            return;
        }
        (*ram).start = start as u32;
        (*ram).length = len as usize;
        (*ram).next = target_get_ram_c(target);
        target_set_ram_c(target, ram);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_add_flash(target: *mut TargetC, flash: *mut c_void) {
    if target.is_null() || flash.is_null() {
        return;
    }
    unsafe { target_flash_setup_c(target, flash) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_ram_map_free(target: *mut TargetC) {
    if target.is_null() {
        return;
    }
    unsafe {
        let mut cur = target_get_ram_c(target);
        while !cur.is_null() {
            let next = (*cur).next;
            target_ram_free_c(cur);
            cur = next;
        }
        target_set_ram_c(target, core::ptr::null_mut());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_flash_map_free(target: *mut TargetC) {
    if target.is_null() {
        return;
    }
    unsafe {
        let mut cur = target_get_flash_c(target);
        while !cur.is_null() {
            let next = target_flash_get_next_c(cur);
            target_flash_buf_free_c(cur);
            cur = next;
        }
        target_set_flash_c(target, core::ptr::null_mut());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem_map_free(target: *mut TargetC) {
    unsafe {
        target_ram_map_free(target);
        target_flash_map_free(target);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_check_error(target: *mut TargetC) -> bool {
    if target.is_null() {
        return false;
    }
    unsafe { target_call_check_error_c(target) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem32_read(
    target: *mut TargetC,
    dest: *mut c_void,
    src: u32,
    len: usize,
) -> bool {
    unsafe { target_mem64_read(target, dest, src as u64, len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem64_read(
    target: *mut TargetC,
    dest: *mut c_void,
    src: u64,
    len: usize,
) -> bool {
    if target.is_null() || dest.is_null() {
        return true;
    }
    unsafe {
        target_call_mem_read_c(target, dest, src, len);
        target_call_check_error_c(target)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem32_write(
    target: *mut TargetC,
    dest: u32,
    src: *const c_void,
    len: usize,
) -> bool {
    unsafe { target_mem64_write(target, dest as u64, src, len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem64_write(
    target: *mut TargetC,
    dest: u64,
    src: *const c_void,
    len: usize,
) -> bool {
    if target.is_null() || src.is_null() {
        return true;
    }
    unsafe {
        target_call_mem_write_c(target, dest, src, len);
        target_call_check_error_c(target)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem32_read32(target: *mut TargetC, addr: u32) -> u32 {
    let mut val: u32 = 0;
    unsafe {
        target_mem32_read(target, &mut val as *mut u32 as *mut c_void, addr, 4);
    }
    val
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem32_write32(target: *mut TargetC, addr: u32, value: u32) -> bool {
    unsafe { target_mem32_write(target, addr, &value as *const u32 as *const c_void, 4) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem32_read16(target: *mut TargetC, addr: u32) -> u16 {
    let mut val: u16 = 0;
    unsafe {
        target_mem32_read(target, &mut val as *mut u16 as *mut c_void, addr, 2);
    }
    val
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem32_write16(target: *mut TargetC, addr: u32, value: u16) -> bool {
    unsafe { target_mem32_write(target, addr, &value as *const u16 as *const c_void, 2) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem32_read8(target: *mut TargetC, addr: u32) -> u8 {
    let mut val: u8 = 0;
    unsafe {
        target_mem32_read(target, &mut val as *mut u8 as *mut c_void, addr, 1);
    }
    val
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem32_write8(target: *mut TargetC, addr: u32, value: u8) -> bool {
    unsafe { target_mem32_write(target, addr, &value as *const u8 as *const c_void, 1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem_access_needs_halt(target: *mut TargetC) -> bool {
    if target.is_null() {
        return true;
    }
    unsafe { target_needs_halt_for_mem_c(target) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem_read_nostop(
    target: *mut TargetC,
    dest: *mut c_void,
    src: u64,
    len: usize,
) -> bool {
    if target.is_null() || dest.is_null() {
        return false;
    }
    unsafe { target_call_mem_read_nostop_c(target, dest, src, len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_mem_write_nostop(
    target: *mut TargetC,
    dest: u64,
    src: *const c_void,
    len: usize,
) -> bool {
    if target.is_null() || src.is_null() {
        return false;
    }
    unsafe { target_call_mem_write_nostop_c(target, dest, src, len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_reg_read(
    target: *mut TargetC,
    reg: u32,
    data: *mut c_void,
    max: usize,
) -> usize {
    if target.is_null() || data.is_null() {
        return 0;
    }
    unsafe { target_call_reg_read_c(target, reg, data, max) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_reg_write(
    target: *mut TargetC,
    reg: u32,
    data: *const c_void,
    size: usize,
) -> usize {
    if target.is_null() || data.is_null() {
        return 0;
    }
    unsafe { target_call_reg_write_c(target, reg, data, size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_regs_read(target: *mut TargetC, data: *mut c_void) {
    if target.is_null() || data.is_null() {
        return;
    }
    unsafe {
        if !target_call_regs_read_c(target, data) {
            let total = target_get_regs_size_c(target);
            let mut offset = 0;
            let mut reg_idx = 0;
            let ptr = data as *mut u8;
            while offset < total {
                let chunk = target_call_reg_read_c(target, reg_idx, ptr.add(offset) as *mut c_void, total - offset);
                if chunk == 0 {
                    break;
                }
                offset += chunk;
                reg_idx += 1;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_regs_write(target: *mut TargetC, data: *const c_void) {
    if target.is_null() || data.is_null() {
        return;
    }
    unsafe {
        if !target_call_regs_write_c(target, data) {
            let total = target_get_regs_size_c(target);
            let mut offset = 0;
            let mut reg_idx = 0;
            let ptr = data as *const u8;
            while offset < total {
                let chunk = target_call_reg_write_c(target, reg_idx, ptr.add(offset) as *const c_void, total - offset);
                if chunk == 0 {
                    break;
                }
                offset += chunk;
                reg_idx += 1;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_regs_size(target: *mut TargetC) -> usize {
    if target.is_null() {
        return 0;
    }
    unsafe { target_get_regs_size_c(target) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_reset(target: *mut TargetC) {
    if !target.is_null() {
        unsafe { target_call_reset_c(target) }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_halt_request(target: *mut TargetC) {
    if !target.is_null() {
        unsafe { target_call_halt_request_c(target) }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_halt_poll(target: *mut TargetC, watch: *mut u64) -> u32 {
    if target.is_null() {
        return 0;
    }
    unsafe { target_call_halt_poll_c(target, watch) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_halt_resume(target: *mut TargetC, step: bool) {
    if !target.is_null() {
        unsafe { target_call_halt_resume_c(target, step) }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_set_cmdline(
    _target: *mut TargetC,
    _cmdline: *const core::ffi::c_char,
    _cmdline_len: usize,
) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_set_heapinfo(
    _target: *mut TargetC,
    _heap_base: u32,
    _heap_limit: u32,
    _stack_base: u32,
    _stack_limit: u32,
) {}
