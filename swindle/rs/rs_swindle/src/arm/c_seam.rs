//! The C seam of the Cortex layers: safe wrappers for target hooks and pure Rust ADIv5 operations.

use core::ffi::c_int;

use super::adi::AdiError;
use super::cortex::AccessPort;
use crate::target::Target;

#[allow(improper_ctypes)]
unsafe extern "C" {
    fn cortexm_probe_chain(target: *mut Target) -> bool;
}

/* --- ADIv5 identification --- */

/// Designer code of the AP's designer (`JEP106`).
pub(crate) fn ap_designer_code(ap: *const AccessPort) -> u32 {
    if ap.is_null() {
        0
    } else {
        unsafe { (*ap).designer_code as u32 }
    }
}

/// Part number of the AP.
pub(crate) fn ap_partno(ap: *const AccessPort) -> u32 {
    if ap.is_null() {
        0
    } else {
        unsafe { (*ap).partno as u32 }
    }
}

/// `DEMCR` of a Cortex-M AP.
pub(crate) fn ap_cortexm_demcr(ap: *const AccessPort) -> u32 {
    if ap.is_null() {
        0
    } else {
        unsafe { (*ap).ap_cortexm_demcr }
    }
}

/// DP protocol version (`0` = DPv0, `1` = DPv1, `2` = DPv2).
pub fn dp_version(ap: *const AccessPort) -> u32 {
    if ap.is_null() || unsafe { (*ap).dp.is_null() } {
        0
    } else {
        unsafe { (*(*ap).dp).version as u32 }
    }
}

/// Designer code of the DP.
pub fn dp_designer_code(ap: *const AccessPort) -> u32 {
    if ap.is_null() || unsafe { (*ap).dp.is_null() } {
        0
    } else {
        unsafe { (*(*ap).dp).designer_code as u32 }
    }
}

/// Designer code of the SoC behind the DP.
pub fn dp_target_designer_code(ap: *const AccessPort) -> u32 {
    if ap.is_null() || unsafe { (*ap).dp.is_null() } {
        0
    } else {
        unsafe { (*(*ap).dp).target_designer_code as u32 }
    }
}

/// Part number of the SoC behind the DP.
pub fn dp_target_partno(ap: *const AccessPort) -> u32 {
    if ap.is_null() || unsafe { (*ap).dp.is_null() } {
        0
    } else {
        unsafe { (*(*ap).dp).target_partno as u32 }
    }
}

/// Last `CTRL/STAT` error value of the DP; clears it.
pub(crate) fn dp_error(ap: *const AccessPort) -> c_int {
    if ap.is_null() || unsafe { (*ap).dp.is_null() } {
        0
    } else {
        unsafe {
            let dp = &mut *(*ap).dp;
            dp.error(false) as c_int
        }
    }
}

/// Read a DP register by number.
pub fn dp_read(ap: *mut AccessPort, reg: u32) -> u32 {
    if ap.is_null() || unsafe { (*ap).dp.is_null() } {
        return 0;
    }
    unsafe {
        use crate::arm::adi::swd::SwdTransport;
        (*(*ap).dp).transport.dp_read(reg as u16)
    }
}

/// Write a DP register by number.
pub fn dp_write(ap: *mut AccessPort, reg: u32, value: u32) {
    if ap.is_null() || unsafe { (*ap).dp.is_null() } {
        return;
    }
    unsafe {
        let _ = (*(*ap).dp).dp_write(reg as u16, value);
    }
}

/// Read an AP register by byte offset.
pub fn ap_read(ap: *mut AccessPort, addr: u16) -> u32 {
    if ap.is_null() {
        return 0;
    }
    unsafe { (*ap).ap_read(addr).unwrap_or(0) }
}

/// Write an AP register by byte offset.
pub fn ap_write(ap: *mut AccessPort, addr: u16, value: u32) {
    if !ap.is_null() {
        unsafe {
            let _ = (*ap).ap_write(addr, value);
        }
    }
}

/* --- DP/AP lifecycle --- */

/// Take a reference on the AP.
pub fn ap_ref(ap: *mut AccessPort) {
    if !ap.is_null() {
        unsafe {
            (*ap).refcnt += 1;
        }
    }
}

/// Drop a reference on the AP.
pub fn ap_unref(ap: *mut AccessPort) {
    if ap.is_null() {
        return;
    }
    unsafe {
        if (*ap).refcnt > 1 {
            (*ap).refcnt -= 1;
        } else {
            let dp_ptr = (*ap).dp;
            crate::session_arena::swindle_session_free(ap.cast());
            if !dp_ptr.is_null() {
                if (*dp_ptr).refcnt > 1 {
                    (*dp_ptr).refcnt -= 1;
                } else {
                    crate::session_arena::swindle_session_free(dp_ptr.cast());
                }
            }
        }
    }
}

/// Forget the AP's cached `CSW`/`TAR`/banked registers.
pub(crate) fn ap_clear_banked(ap: *mut AccessPort) {
    if !ap.is_null() {
        unsafe {
            (*ap).invalidate_caches();
        }
    }
}

/// Invalidate the AP's memory-access caches.
pub(crate) fn ap_invalidate_caches(ap: *mut AccessPort) {
    if !ap.is_null() {
        unsafe {
            (*ap).invalidate_caches();
        }
    }
}

/// Mark the DP as faulted so error recovery switches to this core.
pub(crate) fn dp_set_fault(ap: *mut AccessPort, fault: u32) {
    if !ap.is_null() && !unsafe { (*ap).dp.is_null() } {
        unsafe {
            (*(*ap).dp).fault = fault as u8;
        }
    }
}

/// Wait for the DP to go idle before a direct register access.
pub(crate) fn dp_ensure_idle(_ap: *mut AccessPort) {
    // Synchronous SWD execution ensures line is idle.
}

/* --- memory access through the AP --- */

/// Read `dest.len()` bytes of target memory at `src` into `dest`.
pub(crate) fn ap_mem_read(ap: *mut AccessPort, dest: &mut [u8], src: u32) -> Result<(), AdiError> {
    if ap.is_null() {
        return Err(AdiError::NullPointer);
    }
    if !dest.is_empty() {
        unsafe {
            (*ap).mem_read_bytes(dest, src)?;
        }
    }
    Ok(())
}

/// Write `src` bytes to target memory at `dest`.
pub(crate) fn ap_mem_write(ap: *mut AccessPort, dest: u32, src: &[u8]) -> Result<(), AdiError> {
    if ap.is_null() {
        return Err(AdiError::NullPointer);
    }
    if !src.is_empty() {
        unsafe {
            (*ap).mem_write_bytes(dest, src)?;
        }
    }
    Ok(())
}

/// As [`ap_mem_write()`], forcing the access size (`align`, log2 of the width in bytes).
pub(crate) fn ap_mem_write_aligned(
    ap: *mut AccessPort,
    dest: u32,
    src: &[u8],
    align: u32,
) -> Result<(), AdiError> {
    if ap.is_null() {
        return Err(AdiError::NullPointer);
    }
    if !src.is_empty() {
        unsafe {
            (*ap).mem_write_aligned(dest, src, align)?;
        }
    }
    Ok(())
}

/* --- debug registers --- */

/// Put the AP into the state the debug registers (`DHCSR`, `DCRSR`, …) are accessed in.
pub(crate) fn debug_access_setup(ap: *mut AccessPort) -> Result<(), AdiError> {
    if ap.is_null() {
        return Err(AdiError::NullPointer);
    }
    unsafe {
        (*ap).debug_access_setup()
    }
}

/// Read a core debug register of the SCS window.
pub(crate) fn dp_reg_read(ap: *mut AccessPort, reg: u32) -> Result<u32, AdiError> {
    if ap.is_null() || unsafe { (*ap).dp.is_null() } {
        return Err(AdiError::NullPointer);
    }
    let addr = crate::arm::adi::regs::ADIV5_AP_BD0 + (reg * 4) as u16;
    unsafe {
        (*(*ap).dp).dp_read(addr)
    }
}

/// Write a core debug register of the SCS window.
pub(crate) fn dp_reg_write(ap: *mut AccessPort, reg: u32, value: u32) -> Result<(), AdiError> {
    if ap.is_null() || unsafe { (*ap).dp.is_null() } {
        return Err(AdiError::NullPointer);
    }
    let addr = crate::arm::adi::regs::ADIV5_AP_BD0 + (reg * 4) as u16;
    unsafe {
        (*(*ap).dp).dp_write(addr, value)
    }
}

/* --- target level --- */

/// Let the target driver's own probe chain continue from the Cortex-M one.
pub(crate) fn probe_chain(target: *mut Target) -> bool {
    unsafe { cortexm_probe_chain(target) }
}
