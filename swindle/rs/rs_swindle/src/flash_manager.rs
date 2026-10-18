//! Rust implementation of the Blackmagic target flash manager.
//!
//! Replaces `blackmagic/src/target/target_flash.c`.
//!
//! Provides buffered writing, sector/block erasing, mass erasing, and address-to-flash lookup
//! for attached targets. Interacts with target drivers via C callback function pointers
//! on `TargetFlash`.
//!
//! Note: 64-bit target CPU debugging is explicitly unsupported; addresses are 32-bit (`u32`).

#![allow(unsafe_op_in_unsafe_fn)]

use core::cmp::{max, min};
use cty::c_void;

pub type target_s = c_void;

pub const FLASH_OPERATION_NONE: u8 = 0;
pub const FLASH_OPERATION_ERASE: u8 = 1;
pub const FLASH_OPERATION_MASS_ERASE: u8 = 2;
pub const FLASH_OPERATION_WRITE: u8 = 3;

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
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
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
pub unsafe extern "C" fn flash_buffer_alloc(flash: *mut TargetFlash) -> bool {
    if flash.is_null() {
        return false;
    }
    let f = &mut *flash;
    let buf = malloc(f.writebufsize);
    if buf.is_null() {
        return false;
    }
    f.buf = buf as *mut u8;
    f.buf_addr_base = u32::MAX;
    f.buf_addr_low = u32::MAX;
    f.buf_addr_high = 0;
    true
}

unsafe fn flash_done(flash: *mut TargetFlash) -> bool {
    if flash.is_null() {
        return true;
    }
    let f = &mut *flash;
    if f.operation == FLASH_OPERATION_NONE {
        return true;
    }

    let mut result = true;
    if let Some(done_fn) = f.done {
        result = done_fn(flash);
    }

    if !f.buf.is_null() {
        free(f.buf as *mut c_void);
        f.buf = core::ptr::null_mut();
    }

    f.operation = FLASH_OPERATION_NONE;
    result
}

unsafe fn flash_prepare(flash: *mut TargetFlash, operation: u8) -> bool {
    if flash.is_null() {
        return false;
    }
    let f = &mut *flash;
    if f.operation == operation {
        return true;
    }

    let mut result = true;
    if f.operation != FLASH_OPERATION_NONE {
        result = flash_done(flash);
    }

    if result {
        f.operation = operation;
        if let Some(prepare_fn) = f.prepare {
            result = prepare_fn(flash);
        }
        if !result {
            f.operation = FLASH_OPERATION_NONE;
        }
    }
    result
}

unsafe fn flash_buffered_flush(flash: *mut TargetFlash) -> bool {
    if flash.is_null() {
        return true;
    }
    let f = &mut *flash;
    let mut result = true;

    if !f.buf.is_null()
        && f.buf_addr_base != u32::MAX
        && f.buf_addr_low != u32::MAX
        && f.buf_addr_low < f.buf_addr_high
    {
        if !flash_prepare(flash, FLASH_OPERATION_WRITE) {
            return false;
        }

        let write_fn = match f.write {
            Some(w) => w,
            None => return false,
        };

        let writesize = f.writesize as u32;
        if writesize == 0 {
            return false;
        }
        let aligned_addr = f.buf_addr_low & !(writesize - 1);
        let src_offset = (aligned_addr - f.buf_addr_base) as usize;
        let src = f.buf.add(src_offset);
        let length = f.buf_addr_high - aligned_addr;

        let mut offset = 0u32;
        while offset < length {
            let ok = write_fn(
                flash,
                aligned_addr + offset,
                src.add(offset as usize),
                writesize as usize,
            );
            result &= ok;
            if !result {
                f.buf_addr_base = u32::MAX;
                f.buf_addr_low = u32::MAX;
                f.buf_addr_high = 0;
                return false;
            }
            offset += writesize;
        }

        f.buf_addr_base = u32::MAX;
        f.buf_addr_low = u32::MAX;
        f.buf_addr_high = 0;
    }
    result
}

unsafe fn flash_buffered_write(
    flash: *mut TargetFlash,
    mut dest: u32,
    mut src: *const u8,
    mut len: usize,
) -> bool {
    let f = &mut *flash;
    let writebufsize = f.writebufsize as u32;
    if writebufsize == 0 {
        return false;
    }
    let mut result = true;

    while len > 0 {
        let base_addr = dest & !(writebufsize - 1);

        if base_addr != f.buf_addr_base {
            result &= flash_buffered_flush(flash);
            if !result {
                return false;
            }
            f.buf_addr_base = base_addr;
            core::ptr::write_bytes(f.buf, f.erased, f.writebufsize);
        }

        let offset = (dest % writebufsize) as usize;
        let local_len = min((writebufsize as usize) - offset, len);

        core::ptr::copy_nonoverlapping(src, f.buf.add(offset), local_len);

        f.buf_addr_low = min(f.buf_addr_low, dest);
        f.buf_addr_high = max(f.buf_addr_high, dest + local_len as u32);

        dest += local_len as u32;
        src = src.add(local_len);
        len -= local_len;
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_flash_write(
    target: *mut target_s,
    mut dest: u32,
    mut src: *const u8,
    mut len: usize,
) -> bool {
    if !target_enter_flash_mode_helper(target) {
        return false;
    }

    let mut result = true;
    let mut active_flash: *mut TargetFlash = core::ptr::null_mut();

    let mut flash = target_get_flash(target);
    while !flash.is_null() {
        let f = &*flash;
        let start = f.start;
        let length = f.length as u32;
        if start <= dest && dest < start.wrapping_add(length) {
            active_flash = flash;
        } else if !f.buf.is_null() {
            result &= flash_buffered_flush(flash);
            result &= flash_done(flash);
        }
        flash = f.next;
    }

    if active_flash.is_null() || !result {
        return false;
    }

    while len > 0 {
        let flash = target_flash_for_addr(target, dest);
        if flash.is_null() {
            return false;
        }

        if flash != active_flash {
            result &= flash_buffered_flush(active_flash);
            result &= flash_done(active_flash);
            active_flash = flash;
        }

        if (*flash).buf.is_null() {
            result &= flash_buffer_alloc(flash);
        }

        if !result {
            return false;
        }

        let f = &*flash;
        let end = f.start + f.length as u32;
        let local_end_addr = min(dest + len as u32, end);
        let local_length = (local_end_addr - dest) as usize;

        result &= flash_buffered_write(flash, dest, src, local_length);
        if !result {
            return false;
        }

        dest = local_end_addr;
        src = src.add(local_length);
        len -= local_length;
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_flash_erase(
    target: *mut target_s,
    mut addr: u32,
    mut len: usize,
) -> bool {
    if !target_enter_flash_mode_helper(target) {
        return false;
    }

    let mut active_flash = target_flash_for_addr(target, addr);
    if active_flash.is_null() {
        return false;
    }

    let mut result = true;
    while len > 0 {
        let flash = target_flash_for_addr(target, addr);
        if flash.is_null() {
            return false;
        }

        if flash != active_flash {
            result &= flash_done(active_flash);
            active_flash = flash;
        }

        let f = &*flash;
        let blocksize = f.blocksize as u32;
        if blocksize == 0 {
            return false;
        }
        let local_start_addr = addr & !(blocksize - 1);
        let mut best_blocksize = f.blocksize;
        let padded_len = len + (addr - local_start_addr) as usize;

        if !f.erasesizes.is_null() {
            let mut i = 0;
            loop {
                let s = *f.erasesizes.add(i);
                if s == 0 {
                    break;
                }
                if (local_start_addr & ((s as u32) - 1)) == 0 && padded_len >= s {
                    if s > best_blocksize {
                        best_blocksize = s;
                    }
                }
                i += 1;
            }
        }

        let can_use_mass_erase = f.mass_erase.is_some()
            && local_start_addr == f.start
            && (addr as u64 + len as u64) >= (f.start as u64 + f.length as u64);

        let local_end_addr = if can_use_mass_erase {
            f.start + f.length as u32
        } else {
            local_start_addr + best_blocksize as u32
        };

        let op = if can_use_mass_erase {
            FLASH_OPERATION_MASS_ERASE
        } else {
            FLASH_OPERATION_ERASE
        };
        if !flash_prepare(flash, op) {
            return false;
        }

        let erase_ok = if can_use_mass_erase {
            f.mass_erase.unwrap()(flash, core::ptr::null_mut())
        } else {
            match f.erase {
                Some(erase_fn) => erase_fn(flash, local_start_addr, best_blocksize),
                None => false,
            }
        };
        result &= erase_ok;
        if !result {
            break;
        }

        let advance = min((local_end_addr - addr) as usize, len);
        len -= advance;
        addr = local_end_addr;
    }

    result &= flash_done(active_flash);
    result
}

unsafe fn flash_manual_mass_erase(
    flash: *mut TargetFlash,
    print_progress: *mut platform_timeout_s,
) -> bool {
    let f = &*flash;
    let blocksize = f.blocksize as u32;
    if blocksize == 0 {
        return false;
    }
    let erase_fn = match f.erase {
        Some(e) => e,
        None => return false,
    };
    let mut addr = f.start;
    let end = f.start.wrapping_add(f.length as u32);
    while addr < end {
        if !erase_fn(flash, addr, f.blocksize) {
            return false;
        }
        target_print_progress(print_progress);
        addr = addr.wrapping_add(blocksize);
    }
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_flash_mass_erase(target: *mut target_s) -> bool {
    if !target_enter_flash_mode_helper(target) {
        return false;
    }

    let mut print_progress = platform_timeout_s { time: 0 };
    platform_timeout_set(&mut print_progress, 500);

    let mut result: bool;
    if target_has_mass_erase(target) {
        result = target_mass_erase_call(target, &mut print_progress);
    } else {
        result = true;
        let mut flash = target_get_flash(target);
        while !flash.is_null() {
            let f = &*flash;
            let can_use_mass_erase = f.mass_erase.is_some();

            let op = if can_use_mass_erase {
                FLASH_OPERATION_MASS_ERASE
            } else {
                FLASH_OPERATION_ERASE
            };
            if !flash_prepare(flash, op) {
                result = false;
                break;
            }

            let erase_ok = if can_use_mass_erase {
                f.mass_erase.unwrap()(flash, &mut print_progress)
            } else {
                flash_manual_mass_erase(flash, &mut print_progress)
            };

            result = erase_ok;
            result &= flash_done(flash);
            if !result {
                break;
            }

            flash = f.next;
        }
    }

    target_exit_flash_mode_helper(target);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn target_flash_complete(target: *mut target_s) -> bool {
    if target.is_null() || !target_get_flash_mode(target) {
        return false;
    }

    let mut result = true;
    let mut flash = target_get_flash(target);
    while !flash.is_null() {
        result &= flash_buffered_flush(flash);
        result &= flash_done(flash);
        flash = (*flash).next;
    }

    target_exit_flash_mode_helper(target);
    result
}
