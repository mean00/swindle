//! Cortex-M memory access and D-cache maintenance.

use core::ffi::{c_int, c_void};

use super::c_seam::{ap_mem_read, ap_mem_write, ap_mem_write_aligned};
use super::cortex::check_error;
use super::cortexm::{ap_of, priv_ref};
use super::cortexm_registers::{CORTEXM_DCCIMVAC, CORTEXM_DCCMVAC};
use crate::target::{Target, target_ram_get};

/// Flush the data cache for the RAM regions a transfer touches (C `cortexm_cache_clean()`).
///
/// `invalidate` selects between clean and clean+invalidate. The walk is over the target's
/// RAM regions, so a transfer that touches none of them (every PPB access) does nothing.
pub(super) fn cache_clean(target: *mut Target, addr: u32, len: usize, invalidate: bool) {
    let Some(priv_) = priv_ref(target) else {
        return;
    };
    let (line_length, dcache_enabled) = (priv_.base.dcache_line_length, priv_.dcache_enabled);
    // No data cache, or it is turned off: nothing to maintain.
    if line_length == 0 || !dcache_enabled {
        return;
    }
    let cache_reg = if invalidate {
        CORTEXM_DCCIMVAC
    } else {
        CORTEXM_DCCMVAC
    };
    let minline = u32::from(line_length) << 2;
    // [addr, mem_end); the arithmetic wraps on a range that overflows, which makes the
    // walk below a no-op exactly like the C version.
    let mem_end = addr.wrapping_add(len as u32);

    let mut index = 0;
    while let Some((mut region_start, length)) = target_ram_get(target, index) {
        index += 1;
        let mut region_end = region_start.wrapping_add(length);
        if addr > region_start {
            region_start = addr;
        }
        if mem_end < region_end {
            region_end = mem_end;
        }
        // Walk the intersection line by line, starting at the line boundary below it.
        let mut line = region_start & !(minline - 1);
        while line < region_end {
            write32(target, cache_reg, line);
            line = line.wrapping_add(minline);
        }
    }
}

/// Read `dest.len()` bytes from `src` (C `mem_read_hook()`).
pub(crate) fn mem_read(
    target: *mut Target,
    dest: &mut [u8],
    src: u32,
) -> Result<(), crate::target::TargetError> {
    cache_clean(target, src, dest.len(), false);
    let ap = ap_of(target);
    ap_mem_read(ap, dest, src)?;
    Ok(())
}

/// Write `src` to `dest` (C `mem_write_hook()`).
pub(crate) fn mem_write(
    target: *mut Target,
    dest: u32,
    src: &[u8],
) -> Result<(), crate::target::TargetError> {
    cache_clean(target, dest, src.len(), true);
    let ap = ap_of(target);
    ap_mem_write(ap, dest, src)?;
    Ok(())
}

/// Read a 32-bit word (C `target_mem32_read32()`).
pub(super) fn read32(target: *mut Target, addr: u32) -> u32 {
    let mut value = [0u8; 4];
    let _ = mem_read(target, &mut value, addr);
    u32::from_le_bytes(value)
}

/// Write a 32-bit word (C `target_mem32_write32()`).
pub(super) fn write32(target: *mut Target, addr: u32, value: u32) {
    let _ = mem_write(target, addr, &value.to_le_bytes());
}

/// Read a 16-bit halfword (C `target_mem32_read16()`).
pub(super) fn read16(target: *mut Target, addr: u32) -> u16 {
    let mut value = [0u8; 2];
    let _ = mem_read(target, &mut value, addr);
    u16::from_le_bytes(value)
}

// The `target_s::mem_read` hook of a Cortex-M target (C `cortexm_mem_read`).
// mem_read_hook and mem_write_hook were legacy C vtable hooks; CortexMTargetDriver
// calls mem_read and mem_write directly.

/// Program the target with a forced bus beat width (C `mem_write_aligned()`).
/// Returns non-zero when the access faulted.
pub(crate) fn mem_write_aligned(
    target: *mut Target,
    dest: u32,
    src: *const c_void,
    len: usize,
    align: u32,
) -> c_int {
    if src.is_null() || len == 0 {
        return 1;
    }

    cache_clean(target, dest, len, true);
    let slice = unsafe { core::slice::from_raw_parts(src.cast::<u8>(), len) };
    if ap_mem_write_aligned(ap_of(target), dest, slice, align).is_err() {
        return 1;
    }
    c_int::from(check_error(target))
}
