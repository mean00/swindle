//! Cortex-M banked register file transfers (`DCRSR` / `DCRDR`).

use core::ffi::c_void;

use super::c_seam::{debug_access_setup, dp_reg_read, dp_reg_write};
use super::cortex::{AccessPort, CORTEXM_TOPT_FLAVOUR_FLOAT, CORTEXM_TOPT_TRUSTZONE};
use super::cortexm::ap_of;
use super::cortexm_registers::*;
use crate::target::{Target, TargetView};

/// Wait until DHCSR.S_REGRDY is set (register transfer complete).
fn wait_regrdy(ap: *mut AccessPort) {
    for _ in 0..100 {
        if let Ok(dhcsr) = dp_reg_read(ap, DB_DHCSR) {
            if (dhcsr & CORTEXM_DHCSR_S_REGRDY) != 0 {
                return;
            }
        }
    }
}

/// Read the program counter through banked DCRSR/DCRDR (C `cortexm_pc_read()`).
pub(super) fn pc_read(target: *mut Target) -> u32 {
    let ap = ap_of(target);
    let _ = debug_access_setup(ap);
    let mut value = [0u32; 1];
    read_reg_sequence(ap, &[0x0f], &mut value);
    value[0]
}

/// Write the program counter through banked DCRDR/DCRSR (C `cortexm_pc_write()`).
pub(super) fn pc_write(target: *mut Target, pc: u32) {
    let ap = ap_of(target);
    let _ = debug_access_setup(ap);
    write_reg_sequence(ap, &[0x0f], &[pc]);
}

/// How many 32-bit registers the file holds for a target with `options` (C `regs_size` / 4).
fn reg_count(options: u32) -> usize {
    let mut count = REGNUM_CORTEX_M_GP.len() + REGNUM_CORTEX_M_SP.len() + REGNUM_CORTEX_M_SYS.len();
    if options & CORTEXM_TOPT_TRUSTZONE != 0 {
        count += REGNUM_CORTEX_M_TRUSTZONE.len();
    }
    if options & CORTEXM_TOPT_FLAVOUR_FLOAT != 0 {
        count += REGNUM_CORTEX_MF.len();
    }
    count
}

/// Read one register through DCRSR/DCRDR (C `cortexm_read_reg_sequence()`).
fn read_reg_sequence(ap: *mut AccessPort, map: &[u8], values: &mut [u32]) {
    for (selector, value) in map.iter().zip(values.iter_mut()) {
        let _ = dp_reg_write(ap, DB_DCRSR, u32::from(*selector));
        wait_regrdy(ap);
        *value = dp_reg_read(ap, DB_DCRDR).unwrap_or(0);
    }
}

/// Write one register through DCRDR/DCRSR (C `cortexm_write_reg_sequence()`).
fn write_reg_sequence(ap: *mut AccessPort, map: &[u8], values: &[u32]) {
    for (selector, value) in map.iter().zip(values.iter()) {
        let _ = dp_reg_write(ap, DB_DCRDR, *value);
        let _ = dp_reg_write(ap, DB_DCRSR, CORTEXM_DCRSR_REG_WRITE | u32::from(*selector));
        wait_regrdy(ap);
    }
}

/// Walk every register table, reading into `regs` (C `regs_read_hook()`).
fn regs_read_into(target: *mut Target, regs: &mut [u32]) {
    let ap = ap_of(target);
    let options = TargetView::new(target).options();
    let _ = debug_access_setup(ap);
    let mut offset = 0;
    for table in [&REGNUM_CORTEX_M_GP[..], &REGNUM_CORTEX_M_SP[..], &REGNUM_CORTEX_M_SYS[..]] {
        let end = (offset + table.len()).min(regs.len());
        if offset >= end {
            return;
        }
        read_reg_sequence(ap, table, &mut regs[offset..end]);
        offset = end;
    }
    if options & CORTEXM_TOPT_TRUSTZONE != 0 {
        let end = (offset + REGNUM_CORTEX_M_TRUSTZONE.len()).min(regs.len());
        if offset < end {
            read_reg_sequence(ap, &REGNUM_CORTEX_M_TRUSTZONE, &mut regs[offset..end]);
            offset = end;
        }
    }
    if options & CORTEXM_TOPT_FLAVOUR_FLOAT != 0 {
        let end = (offset + REGNUM_CORTEX_MF.len()).min(regs.len());
        if offset < end {
            read_reg_sequence(ap, &REGNUM_CORTEX_MF, &mut regs[offset..end]);
        }
    }
}

/// Walk every register table, writing from `regs` (C `regs_write_hook()`).
fn regs_write_from(target: *mut Target, regs: &[u32]) {
    let ap = ap_of(target);
    let options = TargetView::new(target).options();
    let _ = debug_access_setup(ap);
    let mut offset = 0;
    for table in [&REGNUM_CORTEX_M_GP[..], &REGNUM_CORTEX_M_SP[..], &REGNUM_CORTEX_M_SYS[..]] {
        let end = (offset + table.len()).min(regs.len());
        if offset >= end {
            return;
        }
        write_reg_sequence(ap, table, &regs[offset..end]);
        offset = end;
    }
    if options & CORTEXM_TOPT_TRUSTZONE != 0 {
        let end = (offset + REGNUM_CORTEX_M_TRUSTZONE.len()).min(regs.len());
        if offset < end {
            write_reg_sequence(ap, &REGNUM_CORTEX_M_TRUSTZONE, &regs[offset..end]);
            offset = end;
        }
    }
    if options & CORTEXM_TOPT_FLAVOUR_FLOAT != 0 {
        let end = (offset + REGNUM_CORTEX_MF.len()).min(regs.len());
        if offset < end {
            write_reg_sequence(ap, &REGNUM_CORTEX_MF, &regs[offset..end]);
        }
    }
}

/// Read the whole register file into `dest` (C `cortexm_regs_read` hook).
pub(crate) fn regs_read_hook(target: *mut Target, dest: *mut c_void) {
    if dest.is_null() {
        return;
    }
    let count = reg_count(TargetView::new(target).options());
    let regs = unsafe { core::slice::from_raw_parts_mut(dest.cast::<u32>(), count) };
    regs_read_into(target, regs);
}

/// Write the whole register file from `data` (C `cortexm_regs_write` hook).
pub(crate) fn regs_write_hook(target: *mut Target, data: *const c_void) {
    if data.is_null() {
        return;
    }
    let count = reg_count(TargetView::new(target).options());
    let regs = unsafe { core::slice::from_raw_parts(data.cast::<u32>(), count) };
    regs_write_from(target, regs);
}

/// GDB register number to DCRSR selector (C `dcrsr_regnum()`): -1 when the register is
/// not part of the file. The `xpsr` gap is why register 25 is expected to be `xpsr`.
fn dcrsr_regnum(target: *mut Target, gdb_reg: u32) -> i32 {
    let mut reg = gdb_reg;
    if reg >= CORTEXM_XPSR_NEW_REGNUM {
        reg = reg + CORTEXM_XPSR_ORIGINAL_REGNUM - CORTEXM_XPSR_NEW_REGNUM;
    }

    for table in [&REGNUM_CORTEX_M_GP[..], &REGNUM_CORTEX_M_SP[..], &REGNUM_CORTEX_M_SYS[..]] {
        if (reg as usize) < table.len() {
            return i32::from(table[reg as usize]);
        }
        reg -= table.len() as u32;
    }

    let options = TargetView::new(target).options();
    if options & CORTEXM_TOPT_TRUSTZONE != 0 {
        if (reg as usize) < REGNUM_CORTEX_M_TRUSTZONE.len() {
            return i32::from(REGNUM_CORTEX_M_TRUSTZONE[reg as usize]);
        }
        reg -= REGNUM_CORTEX_M_TRUSTZONE.len() as u32;
    }
    if options & CORTEXM_TOPT_FLAVOUR_FLOAT != 0 {
        if (reg as usize) < REGNUM_CORTEX_MF.len() {
            return i32::from(REGNUM_CORTEX_MF[reg as usize]);
        }
    }
    -1
}

/// Read one GDB register (C `cortexm_reg_read` hook).
pub(crate) fn reg_read(
    target: *mut Target,
    reg: u32,
    data: *mut c_void,
    max: usize,
) -> usize {
    if data.is_null() || max < core::mem::size_of::<u32>() {
        return 0;
    }
    let r = dcrsr_regnum(target, reg);
    if r < 0 {
        return 0;
    }
    let ap = ap_of(target);
    unsafe {
        let _ = debug_access_setup(ap);
        let selector = r as u8;
        let mut value = [0u32; 1];
        read_reg_sequence(ap, &[selector], &mut value);
        data.cast::<u32>().write_unaligned(value[0]);
    }
    core::mem::size_of::<u32>()
}

/// Write one GDB register (C `cortexm_reg_write` hook).
pub(crate) fn reg_write(
    target: *mut Target,
    reg: u32,
    data: *const c_void,
    max: usize,
) -> usize {
    if data.is_null() || max < core::mem::size_of::<u32>() {
        return 0;
    }
    let r = dcrsr_regnum(target, reg);
    if r < 0 {
        return 0;
    }
    let ap = ap_of(target);
    unsafe {
        let _ = debug_access_setup(ap);
        let selector = r as u8;
        let value = data.cast::<u32>().read_unaligned();
        write_reg_sequence(ap, &[selector], &[value]);
    }
    core::mem::size_of::<u32>()
}
