//! Cortex-M execution control: reset, halt request/poll/resume, fault unwinding,
//! semihosting (`0xbeab`), and the on-target flash stub runner.

use super::c_seam::{ap_invalidate_caches, dp_ensure_idle};
use super::cortex::{CORTEXM_TOPT_FLAVOUR_V8M, check_error};
use super::cortexm::{RESET_TIMEOUT_MS, ap_of, priv_ref, with_priv};
use super::cortexm_breakpoint::check_watch;
use super::cortexm_memaccess::{mem_read, read16, read32, write32};
use super::cortexm_registers::*;
use super::cortexm_regs::{pc_read, pc_write, reg_write, regs_read_hook, regs_write_hook};
use crate::reset_pin;
use crate::target::{HaltReason, Target, TargetView, delay_ms};

/* --- reset --- */

/// `TOPT_INHIBIT_NRST`: the target misbehaves when reset through the nRST line.
const TOPT_INHIBIT_NRST: u32 = 1;

/// Reset the core (C `reset()`).
pub(crate) fn reset(target: *mut Target) {
    let ap = ap_of(target);

    /* Read DHCSR here to clear S_RESET_ST before the reset. */
    let _ = read32(target, CORTEXM_DHCSR);
    /* Use the physical reset pin when the target tolerates it. */
    if TargetView::new(target).options() & TOPT_INHIBIT_NRST == 0 {
        reset_pin::pulse(1);
        /* Some nRF52840 users see invalid SWD transactions without this delay. */
        delay_ms(10);
        dp_ensure_idle(ap);
    }

    /* If no reset was seen, nRST is either not wired or inhibited: ask AIRCR. */
    if read32(target, CORTEXM_DHCSR) & CORTEXM_DHCSR_S_RESET_ST == 0 {
        write32(
            target,
            CORTEXM_AIRCR,
            CORTEXM_AIRCR_VECTKEY | CORTEXM_AIRCR_SYSRESETREQ,
        );
    }

    /* The debug domain may have been reset along with the core: drop the SELECT/CSW
     * write caches so the next access rewrites them. */
    ap_invalidate_caches(ap);

    /* Wait for S_RESET_ST to read 0 again, meaning the reset was released. */
    let mut waited_ms = 0;
    while read32(target, CORTEXM_DHCSR) & CORTEXM_DHCSR_S_RESET_ST != 0 {
        if waited_ms >= RESET_TIMEOUT_MS {
            bmpwarning!("Reset seem to be stuck low!\n");
            break;
        }
        delay_ms(1);
        waited_ms += 1;
    }

    /* Let the clocks (STM32 HSI and friends) start up fully. */
    delay_ms(10);
    /* Reset the DFSR flags and ignore any initial DAP error. */
    write32(target, CORTEXM_DFSR, CORTEXM_DFSR_RESETALL);
}

/* --- halt request / poll / resume --- */

/// Ask the core to halt (C `halt_request()`).
pub(crate) fn halt_request(target: *mut Target) {
    write32(
        target,
        CORTEXM_DHCSR,
        CORTEXM_DHCSR_DBGKEY | CORTEXM_DHCSR_C_HALT | CORTEXM_DHCSR_C_DEBUGEN,
    );
}

/// Resume the core, optionally single-stepping (C `halt_resume()`).
pub(crate) fn halt_resume(target: *mut Target, step: bool) {
    let Some((stepping, on_bkpt, icache_line_length)) = priv_ref(target).map(|priv_| {
        (
            priv_.stepping,
            priv_.on_bkpt,
            priv_.base.icache_line_length,
        )
    }) else {
        return;
    };

    /* Begin building the new DHCSR value to resume the core with. */
    let mut dhcsr = CORTEXM_DHCSR_DBGKEY | CORTEXM_DHCSR_C_DEBUGEN;
    /* Disable interrupts while single stepping. */
    if step {
        dhcsr |= CORTEXM_DHCSR_C_STEP | CORTEXM_DHCSR_C_MASKINTS;
    }

    /*
     * Switching between single-step and run needs C_MASKINTS updated, which is only
     * predictable while C_HALT is set.
     */
    if step != stepping {
        write32(target, CORTEXM_DHCSR, dhcsr | CORTEXM_DHCSR_C_HALT);
        let _ = with_priv(target, |priv_| priv_.stepping = step);
    }

    /* Stepping off a breakpoint instruction must not trap on it again. */
    if on_bkpt {
        let pc = pc_read(target);
        if read16(target, pc) & 0xff00 == 0xbe00 {
            pc_write(target, pc.wrapping_add(2));
        }
    }

    if icache_line_length != 0 {
        write32(target, CORTEXM_ICIALLU, 0);
    }

    /* Release C_HALT to resume the core in whichever mode is selected. */
    write32(target, CORTEXM_DHCSR, dhcsr);
}

/// Why the core stopped, and where a watchpoint matched (C `halt_poll_hook()`).
pub(crate) fn halt_poll(target: *mut Target) -> (HaltReason, u32) {
    if priv_ref(target).is_none() {
        return (HaltReason::Error, 0);
    }

    /* A read that times out means the target is still running (typically in WFI). */
    let dhcsr = read32(target, CORTEXM_DHCSR);
    if dhcsr & CORTEXM_DHCSR_S_HALT == 0 {
        return (HaltReason::Running, 0);
    }

    /* Read and clear the status register to see why we stopped. */
    let dfsr = read32(target, CORTEXM_DFSR);
    write32(target, CORTEXM_DFSR, dfsr);

    /* Remember which caches are enabled: the memory hooks maintain them. */
    let ccr = read32(target, CORTEXM_CCR);
    let (dcache_enabled, icache_enabled) = (
        ccr & CORTEXM_CCR_DCACHE_ENABLE != 0,
        ccr & CORTEXM_CCR_ICACHE_ENABLE != 0,
    );
    let _ = with_priv(target, |priv_| {
        priv_.dcache_enabled = dcache_enabled;
        priv_.icache_enabled = icache_enabled;
    });

    /*
     * On ARMv8-M execution may stop before the faulting instruction retires, so DFSR
     * does not show it: a pending exception in ICSR is the signal there instead.
     */
    let fault = if TargetView::new(target).options() & CORTEXM_TOPT_FLAVOUR_V8M != 0 {
        let pending = (read32(target, CORTEXM_ICSR) >> 12) & 0x1ff;
        // Catch all pending exceptions, but not IRQs.
        pending > 0 && pending < 8
    } else {
        dfsr & CORTEXM_DFSR_VCATCH != 0
    };
    if fault && fault_unwind(target) {
        return (HaltReason::Fault, 0);
    }

    /* Remember whether we stopped on a breakpoint instruction. */
    let on_bkpt = dfsr & CORTEXM_DFSR_BKPT != 0;
    let _ = with_priv(target, |priv_| priv_.on_bkpt = on_bkpt);
    if on_bkpt {
        let pc = pc_read(target);
        let instruction = read16(target, pc);
        /* 0xbeab encodes the breakpoint instruction that marks a semihosting call. */
        if instruction == 0xbeab {
            if hostio_request(target) {
                return (HaltReason::Request, 0);
            }
            /* Resume and let the target carry on with the call handled. */
            let stepping = with_priv(target, |priv_| priv_.stepping).unwrap_or(false);
            halt_resume(target, stepping);
            return (HaltReason::Running, 0);
        }
    }

    if dfsr & CORTEXM_DFSR_DWTTRAP != 0 {
        return (HaltReason::Watchpoint, check_watch(target));
    }
    if dfsr & CORTEXM_DFSR_BKPT != 0 {
        return (HaltReason::Breakpoint, 0);
    }
    if dfsr & CORTEXM_DFSR_HALTED != 0 {
        let stepping = with_priv(target, |priv_| priv_.stepping).unwrap_or(false);
        return (
            if stepping {
                HaltReason::Stepping
            } else {
                HaltReason::Request
            },
            0,
        );
    }
    (HaltReason::Breakpoint, 0)
}

/// Unwind the exception frame a fault left on the stack, so the target can resume at the
/// instruction that faulted with its pre-exception state (C `cortexm_fault_unwind()`).
fn fault_unwind(target: *mut Target) -> bool {
    /* Read the fault status registers, then write them back to clear them. */
    let hfsr = read32(target, CORTEXM_HFSR);
    let cfsr = read32(target, CORTEXM_CFSR);
    write32(target, CORTEXM_HFSR, hfsr);
    write32(target, CORTEXM_CFSR, cfsr);

    /*
     * FORCED in HFSR, or any configurable fault, means a real fault: do not unwind a
     * plain core reset.
     */
    if hfsr & CORTEXM_HFSR_FORCED == 0 && cfsr == 0 {
        return false;
    }

    let mut regs = [0u32; CORTEXM_MAX_REG_COUNT];
    regs_read_hook(target, regs.as_mut_ptr().cast());

    /* The return code in LR says which stack the exception frame is on, and whether
     * the FPU context was stacked with it. */
    let retcode = regs[CORTEX_REG_LR];
    let spsel = retcode & (1 << 2) != 0;
    let fpca = retcode & (1 << 4) == 0;

    let sp = if spsel {
        regs[CORTEX_REG_PSP]
    } else {
        regs[CORTEX_REG_MSP]
    };
    let mut stack_bytes = [0u8; 32];
    if mem_read(target, &mut stack_bytes, sp).is_err() {
        return false;
    }
    let mut stack = [0u32; 8];
    for (value, bytes) in stack.iter_mut().zip(stack_bytes.chunks_exact(4)) {
        *value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    }
    if check_error(target) {
        return false;
    }

    /* Restore LR and PC to their pre-exception values. */
    regs[CORTEX_REG_LR] = stack[5];
    regs[CORTEX_REG_PC] = stack[6];

    /* Pop the exception frame: basic (0x20) or extended with FPU state (0x68). */
    let mut frame_size = if fpca { 0x68 } else { 0x20 };
    /* The frame is padded when the stack was not 8-byte aligned. */
    if stack[7] & (1 << 9) != 0 {
        frame_size += 4;
    }

    if spsel {
        regs[CORTEX_REG_PSP] = regs[CORTEX_REG_PSP].wrapping_add(frame_size);
        regs[CORTEX_REG_SP] = regs[CORTEX_REG_PSP];
        regs[CORTEX_REG_SPECIAL] |= CORTEX_SPECIAL_SPSEL;
    } else {
        regs[CORTEX_REG_MSP] = regs[CORTEX_REG_MSP].wrapping_add(frame_size);
        regs[CORTEX_REG_SP] = regs[CORTEX_REG_MSP];
    }
    if fpca {
        regs[CORTEX_REG_SPECIAL] |= CORTEX_SPECIAL_FPCA;
    }

    /* Clear the active exception state so the restored state can resume. */
    write32(
        target,
        CORTEXM_AIRCR,
        CORTEXM_AIRCR_VECTKEY | CORTEXM_AIRCR_VECTCLRACTIVE,
    );

    /* Write the pre-exception registers back to the core. */
    regs_write_hook(target, regs.as_ptr().cast());
    true
}

/// Hand a semihosting call to the host and write the result back
/// (C `cortexm_hostio_request()`). Returns true when the request was interrupted.
fn hostio_request(target: *mut Target) -> bool {
    // Semihosting is unsupported; write -1 (failure) to r0 so firmware can continue.
    let result = -1i32;
    reg_write(
        target,
        0,
        (&raw const result).cast(),
        core::mem::size_of::<i32>(),
    );
    TargetView::new(target).interrupted()
}


