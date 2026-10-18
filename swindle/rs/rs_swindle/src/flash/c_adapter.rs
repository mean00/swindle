//! C-ABI translation layer adapting C target_flash drivers to the idiomatic Rust FlashManager.

#![allow(unsafe_op_in_unsafe_fn)]

use alloc::vec::Vec;
use cty::c_void;
use super::driver::{FlashDriver, FlashError, FlashOperation};
use super::geometry::FlashGeometry;
use super::manager::{FlashManager, FlashRegion};

pub type target_s = c_void;

#[repr(C)]
pub struct platform_timeout_s {
    pub time: u32,
}

#[repr(C)]
pub struct TargetFlash {
    pub t: *mut target_s,
    pub start: u32,
    pub length: usize,
    pub blocksize: usize,
    pub erasesizes: *const usize,
    pub writesize: usize,
    pub writebufsize: usize,
    pub erased: u8,
    pub operation: u8,
    pub prepare: Option<unsafe extern "C" fn(flash: *mut TargetFlash) -> bool>,
    pub erase: Option<unsafe extern "C" fn(flash: *mut TargetFlash, addr: u32, len: usize) -> bool>,
    pub mass_erase: Option<unsafe extern "C" fn(flash: *mut TargetFlash, timeout: *mut platform_timeout_s) -> bool>,
    pub write: Option<unsafe extern "C" fn(flash: *mut TargetFlash, dest: u32, src: *const u8, len: usize) -> bool>,
    pub done: Option<unsafe extern "C" fn(flash: *mut TargetFlash) -> bool>,
    pub buf: *mut u8,
    pub buf_addr_base: u32,
    pub buf_addr_low: u32,
    pub buf_addr_high: u32,
    pub next: *mut TargetFlash,
}

unsafe extern "C" {
    fn platform_timeout_set(timeout: *mut platform_timeout_s, ms: u32);
    fn target_print_progress(timeout: *mut platform_timeout_s);

    // Helpers from bmp_helpers.cpp
    fn target_get_flash(target: *mut target_s) -> *mut TargetFlash;
    fn target_get_flash_mode(target: *mut target_s) -> bool;
    fn target_enter_flash_mode_helper(target: *mut target_s) -> bool;
    fn target_exit_flash_mode_helper(target: *mut target_s) -> bool;
    fn target_has_mass_erase(target: *mut target_s) -> bool;
    fn target_mass_erase_call(target: *mut target_s, timeout: *mut platform_timeout_s) -> bool;
}

/// Implements the idiomatic `FlashDriver` trait by dispatching to C callbacks in `TargetFlash`.
pub struct CTargetFlashDriver {
    pub flash: *mut TargetFlash,
}

impl FlashDriver for CTargetFlashDriver {
    fn prepare(&mut self, op: FlashOperation) -> Result<(), FlashError> {
        unsafe {
            let f = &mut *self.flash;
            if f.operation == op as u8 {
                return Ok(());
            }

            if f.operation != FlashOperation::None as u8 {
                self.done()?;
            }

            f.operation = op as u8;
            if let Some(cb) = f.prepare {
                if !cb(self.flash) {
                    f.operation = FlashOperation::None as u8;
                    return Err(FlashError::DriverFailed);
                }
            }
            Ok(())
        }
    }

    fn erase_block(&mut self, addr: u32, len: usize) -> Result<(), FlashError> {
        unsafe {
            let f = &*self.flash;
            match f.erase {
                Some(cb) if cb(self.flash, addr, len) => Ok(()),
                _ => Err(FlashError::DriverFailed),
            }
        }
    }

    fn has_mass_erase(&self) -> bool {
        unsafe { (*self.flash).mass_erase.is_some() }
    }

    fn mass_erase(&mut self) -> Result<(), FlashError> {
        unsafe {
            let f = &*self.flash;
            match f.mass_erase {
                Some(cb) if cb(self.flash, core::ptr::null_mut()) => Ok(()),
                _ => Err(FlashError::DriverFailed),
            }
        }
    }

    fn write_block(&mut self, dest: u32, data: &[u8]) -> Result<(), FlashError> {
        unsafe {
            let f = &*self.flash;
            match f.write {
                Some(cb) if cb(self.flash, dest, data.as_ptr(), data.len()) => Ok(()),
                _ => Err(FlashError::DriverFailed),
            }
        }
    }

    fn done(&mut self) -> Result<(), FlashError> {
        unsafe {
            let f = &mut *self.flash;
            if f.operation == FlashOperation::None as u8 {
                return Ok(());
            }

            let mut res = Ok(());
            if let Some(cb) = f.done {
                if !cb(self.flash) {
                    res = Err(FlashError::DriverFailed);
                }
            }
            f.operation = FlashOperation::None as u8;
            res
        }
    }
}

/// Static state for active flash programming session.
static mut ACTIVE_MANAGER: Option<FlashManager<CTargetFlashDriver>> = None;
static mut ACTIVE_TARGET: *mut target_s = core::ptr::null_mut();

unsafe fn get_or_create_manager(target: *mut target_s) -> Option<&'static mut FlashManager<CTargetFlashDriver>> {
    if target.is_null() {
        return None;
    }

    if let Some(ref mut mgr) = ACTIVE_MANAGER {
        if ACTIVE_TARGET == target {
            return Some(mgr);
        }
        // Target changed: finalize previous manager
        let _ = mgr.complete();
        ACTIVE_MANAGER = None;
        ACTIVE_TARGET = core::ptr::null_mut();
    }

    let mut mgr = FlashManager::new();
    let mut flash_ptr = target_get_flash(target);
    while !flash_ptr.is_null() {
        let f = &*flash_ptr;
        let geom = FlashGeometry::new(
            f.start,
            f.length,
            f.blocksize,
            f.writesize,
            f.writebufsize,
            f.erased,
        );

        let mut extra_sizes = Vec::new();
        if !f.erasesizes.is_null() {
            let mut i = 0;
            loop {
                let sz = *f.erasesizes.add(i);
                if sz == 0 {
                    break;
                }
                extra_sizes.push(sz);
                i += 1;
            }
        }

        let driver = CTargetFlashDriver { flash: flash_ptr };
        let region = FlashRegion::new(geom, extra_sizes, driver);
        mgr.add_region(region);

        flash_ptr = f.next;
    }

    ACTIVE_TARGET = target;
    ACTIVE_MANAGER = Some(mgr);
    ACTIVE_MANAGER.as_mut()
}

// ---------------------------------------------------------------------------
// Exported C-ABI Functions
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_flash_for_addr(
    target: *mut target_s,
    addr: u32,
) -> *mut TargetFlash {
    if target.is_null() {
        return core::ptr::null_mut();
    }
    let mut flash = target_get_flash(target);
    while !flash.is_null() {
        let f = &*flash;
        let start = f.start;
        let len = f.length as u32;
        if start <= addr && addr < start.wrapping_add(len) {
            return flash;
        }
        flash = f.next;
    }
    core::ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flash_buffer_alloc(_flash: *mut TargetFlash) -> bool {
    // Buffering is now completely managed by FlashSectorBuffer in Rust.
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_flash_erase(
    target: *mut target_s,
    addr: u32,
    len: usize,
) -> bool {
    if !target_enter_flash_mode_helper(target) {
        return false;
    }

    let mgr = match get_or_create_manager(target) {
        Some(m) => m,
        None => return false,
    };

    mgr.erase(addr, len).is_ok()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_flash_write(
    target: *mut target_s,
    dest: u32,
    src: *const u8,
    len: usize,
) -> bool {
    if !target_enter_flash_mode_helper(target) {
        return false;
    }

    if src.is_null() && len > 0 {
        return false;
    }

    let slice = core::slice::from_raw_parts(src, len);
    let mgr = match get_or_create_manager(target) {
        Some(m) => m,
        None => return false,
    };

    mgr.write(dest, slice).is_ok()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_flash_complete(target: *mut target_s) -> bool {
    if target.is_null() || !target_get_flash_mode(target) {
        return false;
    }

    let mut result = true;
    if let Some(ref mut mgr) = ACTIVE_MANAGER {
        result = mgr.complete().is_ok();
    }
    ACTIVE_MANAGER = None;
    ACTIVE_TARGET = core::ptr::null_mut();

    target_exit_flash_mode_helper(target);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_flash_mass_erase(target: *mut target_s) -> bool {
    if !target_enter_flash_mode_helper(target) {
        return false;
    }

    let mut print_progress = platform_timeout_s { time: 0 };
    platform_timeout_set(&mut print_progress, 500);

    let result = if target_has_mass_erase(target) {
        target_mass_erase_call(target, &mut print_progress)
    } else {
        match get_or_create_manager(target) {
            Some(mgr) => {
                mgr.mass_erase_all(|| {
                    target_print_progress(&mut print_progress);
                })
                .is_ok()
            }
            None => false,
        }
    };

    target_exit_flash_mode_helper(target);
    result
}
