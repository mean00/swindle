//! Cortex-M debug layer: the Rust half of `blackmagic/src/target/cortexm.c`.
//!
//! Everything the probe does to a Cortex-M core that is not ADIv5 plumbing lives in this
//! module and its companion sub-modules:
//! - [`super::cortexm_memaccess`]: memory reads/writes and D-cache maintenance
//! - [`super::cortexm_regs`]: banked register file transfers (`DCRSR` / `DCRDR`)
//! - [`super::cortexm_breakpoint`]: FPB hardware breakpoints and DWT watchpoints
//! - [`super::cortexm_halt`]: reset, halt request/poll/resume, fault unwinding, semihosting,
//!   and the flash stub runner
//! - `cortexm` (this file): private state (`CortexMPriv`), `probe`/`attach`/`detach`,
//!   `vector_catch`, and [`CortexMTargetDriver`]

use core::ffi::c_void;

use super::c_seam::*;
use super::cortex::{
    AccessPort, CortexPriv, CORTEXM_TOPT_FLAVOUR_FLOAT, CORTEXM_TOPT_FLAVOUR_V6M,
    CORTEXM_TOPT_TRUSTZONE, cpuid_flavour,
};
use super::cortexm_breakpoint::{cortexm_dwt_func, cortexm_fpb_comp};
use super::cortexm_memaccess::{read32, write32};
use super::cortexm_registers::*;
use crate::reset_pin;
use crate::session_arena::{swindle_session_calloc, swindle_session_free};
use crate::target::breakwatch::{Breakwatch, BreakwatchTable};
use crate::target::{HaltReason, Target, TargetError, TargetView, delay_ms, target_new};

// Re-export sub-module functions so `c_api` and crate-internal callers continue to use `cortexm::*`.
pub(crate) use super::cortexm_breakpoint::{
    breakwatch_clear, breakwatch_clear_all, breakwatch_set,
};
pub(crate) use super::cortexm_halt::{
    halt_poll, halt_request, halt_resume, reset,
};
pub(crate) use super::cortexm_memaccess::{
    mem_read, mem_write, mem_write_aligned,
};
pub(crate) use super::cortex::check_error;
pub(crate) use super::cortexm_regs::{reg_read, reg_write, regs_read_hook, regs_write_hook};

/// The Cortex-M private state: C `cortexm_priv_s`.
///
/// `target->priv` points at this; the [`CortexPriv`] prefix is what the generic Cortex
/// layer's exported functions find through it.
#[repr(C)]
pub struct CortexMPriv {
    /// The generic Cortex state (access port, debug base address, cache and slot masks).
    pub base: CortexPriv,
    /// The core was resumed with `C_STEP`: a halt is a completed single step.
    pub(super) stepping: bool,
    /// The last halt was on a breakpoint instruction (the PC may need nudging on resume).
    pub(super) on_bkpt: bool,
    /// `CCR.ICACHE` as last read while halted.
    pub(super) icache_enabled: bool,
    /// `CCR.DCACHE` as last read while halted.
    pub(super) dcache_enabled: bool,
    /// Revision of the FPB unit (0 = ARMv7-M FPB, limited to the first 512 MiB).
    pub(super) flash_patch_revision: u8,
    /// Copy of DEMCR, so `vector_catch` can flip bits in it.
    pub(super) demcr: u32,
    /// The hardware triggers this core has armed, with their comparator slots.
    pub(super) breakwatches: BreakwatchTable,
}

impl CortexMPriv {
    fn new(ap: *mut AccessPort) -> Self {
        Self {
            base: CortexPriv::new(ap, CORTEXM_SCS_BASE),
            stepping: false,
            on_bkpt: false,
            icache_enabled: false,
            dcache_enabled: false,
            flash_patch_revision: 0,
            demcr: 0,
            breakwatches: BreakwatchTable::new(),
        }
    }
}

/// The Cortex-M state of `target`, or null when it carries none.
fn state(target: *mut Target) -> *mut CortexMPriv {
    CortexPriv::of(target).cast()
}

/// The Cortex-M state of `target`, or `None` when it carries none.
///
/// The borrow is as long as the target's: C allocates the state in `probe()` and only
/// `priv_free()` releases it, so it outlives every borrow. It is read-only and meant to be
/// short — keep it out of calls that re-enter C and read the state again.
pub(super) fn priv_ref(target: *mut Target) -> Option<&'static CortexMPriv> {
    // Safety: `state()` is either null (→ `None`) or the live allocation `target_s` owns.
    unsafe { state(target).as_ref() }
}

/// Run `f` on the Cortex-M state of `target`, or return `None` when it carries none.
///
/// This is how the layer changes its state without an `unsafe` block per field: the raw deref
/// lives here, and the mutable borrow cannot outlive the call.
pub(super) fn with_priv<R>(target: *mut Target, f: impl FnOnce(&mut CortexMPriv) -> R) -> Option<R> {
    // Safety: as in `priv_ref()`, and the borrow ends when `f` returns.
    unsafe { state(target).as_mut() }.map(f)
}

/// Access port of `target`, or null.
pub(super) fn ap_of(target: *mut Target) -> *mut AccessPort {
    priv_ref(target).map_or(core::ptr::null_mut(), |priv_| priv_.base.ap)
}

/// The DEMCR the target runs with, as last set (C `demcr_read()`).
#[allow(dead_code)]
pub(crate) fn demcr_read(target: *const Target) -> u32 {
    priv_ref(target.cast_mut()).map_or(0, |priv_| priv_.demcr)
}

/// Update the DEMCR copy and the target (C `demcr_write()`).
#[allow(dead_code)]
pub(crate) fn demcr_write(target: *mut Target, demcr: u32) {
    if with_priv(target, |priv_| priv_.demcr = demcr).is_none() {
        return;
    }
    write32(target, CORTEXM_DEMCR, demcr);
}

/// True when `target` is driven by this Cortex-M layer (C `is_cortexm()`).
pub(crate) fn is_cortexm(target: *const Target) -> bool {
    TargetView::new(target.cast_mut()).is_cortexm()
}

/// Release the private state of a Cortex-M target.
pub(crate) unsafe fn priv_free(priv_: *mut c_void) {
    if priv_.is_null() {
        return;
    }
    let state = priv_.cast::<CortexMPriv>();
    unsafe {
        ap_unref((*state).base.ap);
        // Session-arena `free()`: rewinds the block when it is the top one.
        swindle_session_free(priv_);
    }
}

/* --- probe --- */

/// `JEP106_MANUFACTURER_ARM_CHINA`: the designer code errata below maps onto.
const JEP106_MANUFACTURER_ARM_CHINA: u32 = 0xa75;
/// `JEP106_MANUFACTURER_ERRATA_ARM_CHINA`: what MM32F5xxx parts report instead.
const JEP106_MANUFACTURER_ERRATA_ARM_CHINA: u32 = 0xc7f;

/// How long the connect-under-reset path waits for the core, in ms (C timeout: 1000 ms).
pub(super) const RESET_TIMEOUT_MS: u32 = 1000;

/// Probe a Cortex-M core behind `ap`, create its `target_s` and hand it to the driver chain
/// (C `probe()`).
pub(crate) fn probe(ap: *mut AccessPort) -> bool {
    if ap.is_null() {
        return false;
    }
    let target = unsafe { target_new() };
    if target.is_null() {
        return false;
    }
    let view = TargetView::new(target);

    view.set_arch_arm();
    ap_ref(ap);

    /* DPv2+ carries a TARGETID register identifying the part directly; older debug
     * ports only have the AP's DESIGNER/PARTNO. */
    let dp_has_targetid =
        dp_version(ap) >= 2 && dp_target_designer_code(ap) != 0;
    let (mut designer, part_id) = if dp_has_targetid {
        (dp_target_designer_code(ap), dp_target_partno(ap))
    } else {
        (ap_designer_code(ap), ap_partno(ap))
    };
    /* MM32F5xxx: the part reports Arm China through a forbidden continuation code. */
    if designer == JEP106_MANUFACTURER_ERRATA_ARM_CHINA
        && dp_designer_code(ap) == JEP106_MANUFACTURER_ARM_CHINA
    {
        designer = JEP106_MANUFACTURER_ARM_CHINA;
    }
    view.set_designer_code(designer);
    view.set_part_id(part_id);

    // Safety: the C allocator returns a zeroed block of that size or null, which the
    // check below handles.
    let state_ptr = unsafe { swindle_session_calloc(1, core::mem::size_of::<CortexMPriv>()) }
        .cast::<CortexMPriv>();
    if state_ptr.is_null() {
        bmpwarning!("calloc: failed in cortexm_probe\n");
        return false;
    }
    // Safety: the arena block holds exactly `size_of::<CortexMPriv>()` bytes and
    // nothing else refers to it yet.
    unsafe { state_ptr.write(CortexMPriv::new(ap)) };
    view.set_priv(state_ptr);
    view.set_priv_free(priv_free);
    view.set_driver("ARM Cortex-M");
    view.set_is_cortexm(true);

    /* `cpuid_flavour` ORs the flavour bits in itself; everything decoded below is
     * added to the same flag word instead of replacing it. */
    let mut options = cpuid_flavour(target);

    /* The security extension is not part of ARMv6-M. */
    if options & CORTEXM_TOPT_FLAVOUR_V6M == 0
        && read32(target, CORTEXM_ID_PFR1) & CORTEXM_ID_PFR1_SECEXT_IMPL != 0
    {
        options |= CORTEXM_TOPT_TRUSTZONE;
        view.or_options(CORTEXM_TOPT_TRUSTZONE);
    }

    /* CPACR only keeps the CP10/CP11 enable bits when the core has an FPU. */
    let cpacr = read32(target, CORTEXM_CPACR) | 0x00f0_0000;
    write32(target, CORTEXM_CPACR, cpacr);
    let has_fpu = read32(target, CORTEXM_CPACR) == cpacr;

    let mut regs_size = core::mem::size_of::<u32>() * CORTEXM_GENERAL_REG_COUNT;
    if options & CORTEXM_TOPT_TRUSTZONE != 0 {
        regs_size += core::mem::size_of::<u32>() * CORTEXM_TRUSTZONE_REG_COUNT;
    }
    if has_fpu {
        // Only the target's flag word has to record the FPU; the register file size
        // below already accounts for it.
        view.or_options(CORTEXM_TOPT_FLAVOUR_FLOAT);
        regs_size += core::mem::size_of::<u32>() * CORTEX_FLOAT_REG_COUNT;
    }
    view.set_regs_size(regs_size);

    /* Default vectors to catch: hard faults and the reset vector. */
    let _ = with_priv(target, |priv_| {
        priv_.demcr = CORTEXM_DEMCR_TRCENA | CORTEXM_DEMCR_VC_HARDERR | CORTEXM_DEMCR_VC_CORERESET;
    });

    /*
     * Some devices, such as the STM32F0, do not respond to probes under reset. When
     * connecting under reset, catch the reset vector first and only then release the
     * reset, so no user code runs before the core is halted.
     */
    let mut conn_reset = false;
    if reset_pin::is_asserted() {
        conn_reset = true;
        let demcr = priv_ref(target).map_or(0, |priv_| priv_.demcr);
        write32(target, CORTEXM_DEMCR, demcr);
        halt_request(target);
        reset_pin::release();
        let mut waited_ms = 0;
        while read32(target, CORTEXM_DHCSR) & CORTEXM_DHCSR_S_RESET_ST != 0 {
            if waited_ms >= RESET_TIMEOUT_MS {
                bmpwarning!("Error releasing from reset\n");
                // Carry on and try to detect the target anyway.
                break;
            }
            delay_ms(1);
            waited_ms += 1;
        }
    }

    /* Cache type: ARMv7 cores report their cache line lengths in CTR. */
    let cache_type = read32(target, CORTEXM_CTR);
    if cache_type >> CORTEX_CTR_FORMAT_SHIFT == CORTEX_CTR_FORMAT_ARMV7 {
        let icache_line_length = (1u32 << (cache_type & CORTEX_CTR_ICACHE_LINE_MASK)) as u16;
        let dcache_line_length = (1u32
            << ((cache_type >> CORTEX_CTR_DCACHE_LINE_SHIFT) & CORTEX_CTR_DCACHE_LINE_MASK))
            as u16;
        let _ = with_priv(target, |priv_| {
            priv_.base.icache_line_length = icache_line_length;
            priv_.base.dcache_line_length = dcache_line_length;
        });
    } else {
        check_error(target);
    }

    /* If the interrupt catch vector was set above, clear it again. */
    if conn_reset {
        write32(target, CORTEXM_DEMCR, 0);
    }

    /* Which driver gets the target is the Rust probe chain of the ARM drivers; a family
     * that is not ported yet is called back into its C probe from there. */
    if probe_chain(target) {
        return true;
    }
    /* Nothing claimed the target: say so, as the C probe chain does. */
    let (designer_code, partno) = (ap_designer_code(ap), ap_partno(ap));
    gdb_println!(
        "Please report unknown device with Designer ",
        Hex(designer_code),
        " Part ID ",
        Hex(partno)
    );
    true
}

/// How long attach waits for the core to report halted, in ms (C timeout: 250 ms).
const ATTACH_HALT_TIMEOUT_MS: u32 = 250;

/// Attach to a Cortex-M core: halt it, size up its breakpoint units and arm them
/// (C `attach()`).
pub(crate) fn attach(target: *mut Target) -> bool {
    if priv_ref(target).is_none() {
        return false;
    }
    let ap = ap_of(target);

    /* Mark the DP as faulted so error recovery switches to this core in multi-drop
     * mode, then clear any pending fault condition (which also switches to it). */
    dp_set_fault(ap, 1);
    check_error(target);

    /* Try to halt the core, and check that it worked (which also resets the reason). */
    halt_request(target);
    let mut reason = HaltReason::Running;
    let mut waited_ms = 0;
    while reason == HaltReason::Running && waited_ms < ATTACH_HALT_TIMEOUT_MS {
        reason = halt_poll(target).0;
        if reason == HaltReason::Running {
            delay_ms(1);
            waited_ms += 1;
        }
    }
    /* If we did not succeed, abort: nothing below can be trusted. */
    if reason == HaltReason::Fault || reason == HaltReason::Error {
        return false;
    }

    /* Request a halt on reset. */
    if let Some(demcr) = priv_ref(target).map(|priv_| priv_.demcr) {
        write32(target, CORTEXM_DEMCR, demcr);
    }

    /* Find out how many breakpoint slots there are. The full 7-bit NUM_CODE field is
     * reconstructed so parts with more slots than we handle do not get truncated. */
    let flash_break_cfg = read32(target, CORTEXM_FPB_CTRL);
    let breakpoints = (((flash_break_cfg & CORTEXM_FPB_CTRL_NUM_CODE_H) >> 8)
        | ((flash_break_cfg & CORTEXM_FPB_CTRL_NUM_CODE_L) >> 4)) as u8;
    /* The FPB revision changes how a comparator value is encoded. */
    let revision =
        ((flash_break_cfg >> CORTEXM_FPB_CTRL_REV_SHIFT) & CORTEXM_FPB_CTRL_REV_MASK) as u8;

    /* Now find out how many watchpoint slots there are. */
    let watchpoints = (read32(target, CORTEXM_DWT_CTRL) >> 28) as u8;

    let Some((breakpoints, watchpoints)) = with_priv(target, |priv_| {
        priv_.base.breakpoints_available = CORTEX_MAX_BREAKPOINTS.min(breakpoints);
        priv_.flash_patch_revision = revision;
        priv_.base.watchpoints_available = CORTEX_MAX_WATCHPOINTS.min(watchpoints);
        (
            priv_.base.breakpoints_available,
            priv_.base.watchpoints_available,
        )
    }) else {
        return false;
    };

    bmpwarning!(
        "Cortex-M core has {} breakpoint and {} watchpoint slots available\n",
        breakpoints,
        watchpoints
    );

    /* Clear any stale breakpoints and watchpoints. */
    for slot in 0..u32::from(breakpoints) {
        write32(target, cortexm_fpb_comp(slot), 0);
    }
    for slot in 0..u32::from(watchpoints) {
        write32(target, cortexm_dwt_func(slot), 0);
    }
    let _ = with_priv(target, |priv_| {
        priv_.base.breakpoints_mask = 0;
        priv_.base.watchpoints_mask = 0;
    });

    /* Flash Patch Control Register: set ENABLE. */
    write32(target, CORTEXM_FPB_CTRL, CORTEXM_FPB_CTRL_KEY | CORTEXM_FPB_CTRL_ENABLE);

    /* If the core is still in reset, release it and wait for the status to clear. */
    let _ = read32(target, CORTEXM_DHCSR);
    if read32(target, CORTEXM_DHCSR) & CORTEXM_DHCSR_S_RESET_ST != 0 {
        reset_pin::release();
        let mut waited_ms = 0;
        loop {
            if read32(target, CORTEXM_DHCSR) & CORTEXM_DHCSR_S_RESET_ST == 0 {
                break;
            }
            if waited_ms >= RESET_TIMEOUT_MS {
                bmpwarning!("Error releasing from reset\n");
                return false;
            }
            delay_ms(1);
            waited_ms += 1;
        }
    }
    true
}

/// Detach from a Cortex-M core: drop the breakpoint units, restore DEMCR and let the core
/// run (C `detach()`).
pub(crate) fn detach(target: *mut Target) {
    let Some((breakpoints, watchpoints)) = priv_ref(target).map(|priv_| {
        (
            priv_.base.breakpoints_available,
            priv_.base.watchpoints_available,
        )
    }) else {
        return;
    };
    let ap = ap_of(target);

    /* Clear any stale breakpoints and watchpoints. */
    for slot in 0..u32::from(breakpoints) {
        write32(target, cortexm_fpb_comp(slot), 0);
    }
    for slot in 0..u32::from(watchpoints) {
        write32(target, cortexm_dwt_func(slot), 0);
    }
    /* Restore the DEMCR the target had when we attached. */
    write32(target, CORTEXM_DEMCR, ap_cortexm_demcr(ap));
    /* Resume the target and disable debug, re-enabling interrupts in the process. */
    write32(
        target,
        CORTEXM_DHCSR,
        CORTEXM_DHCSR_DBGKEY | CORTEXM_DHCSR_C_DEBUGEN | CORTEXM_DHCSR_C_HALT,
    );
    write32(target, CORTEXM_DHCSR, CORTEXM_DHCSR_DBGKEY | CORTEXM_DHCSR_C_DEBUGEN);
    write32(target, CORTEXM_DHCSR, CORTEXM_DHCSR_DBGKEY);
}

/* --- `mon vector_catch` --- */

/// Exception vectors `vector_catch` can arm, by DEMCR bit (C `vectors[]`); the gaps are
/// the vectors the command does not expose.
const VECTOR_NAMES: [Option<&core::ffi::CStr>; 12] = [
    Some(c"reset"),
    None,
    None,
    None,
    Some(c"mm"),
    Some(c"nocp"),
    Some(c"chk"),
    Some(c"stat"),
    Some(c"bus"),
    Some(c"int"),
    Some(c"hard"),
    Some(c"sf"),
];

/// Vector catch command invoked by `mon vector_catch`.
pub fn vector_catch_cmd(target: *mut Target, args: &[&str]) -> bool {
    if priv_ref(target).is_none() {
        return false;
    }
    if args.len() < 2 {
        crate::gdb_println!(
            "usage: monitor vector_catch (enable|disable) (sf|hard|int|bus|stat|chk|nocp|mm|reset)"
        );
    } else {
        let enable = if "enable".starts_with(args[0]) {
            true
        } else if "disable".starts_with(args[0]) {
            false
        } else {
            crate::gdb_println!("'enable' or 'disable' argument must be provided");
            return false;
        };

        let mut mask = 0u32;
        for &arg in &args[1..] {
            for (bit, name) in VECTOR_NAMES.iter().enumerate() {
                if let Some(name) = name
                    && name.to_bytes() == arg.as_bytes()
                {
                    mask |= 1 << bit;
                }
            }
        }

        let demcr = with_priv(target, |priv_| {
            if enable {
                priv_.demcr |= mask;
            } else {
                priv_.demcr &= !mask;
            }
            priv_.demcr
        });
        if let Some(demcr) = demcr {
            write32(target, CORTEXM_DEMCR, demcr);
        }
    }

    let demcr = priv_ref(target).map_or(0, |priv_| priv_.demcr);
    crate::gdb_print!("Catching vectors: ");
    for (bit, name) in VECTOR_NAMES.iter().enumerate() {
        if let Some(name) = name
            && demcr & (1 << bit) != 0
        {
            if let Ok(s) = name.to_str() {
                crate::gdb_print!("{} ", s);
            }
        }
    }
    crate::gdb_println!("");
    true
}

/// First-class Rust `TargetDriver` for pure-Rust Cortex-M targets.
pub struct CortexMTargetDriver {
    pub target: *mut Target,
}

impl crate::target::TargetDriver for CortexMTargetDriver {
    fn mem_read(&self, dest: &mut [u8], addr: u32) -> Result<(), TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        mem_read(self.target, dest, addr)?;
        if check_error(self.target) {
            Err(TargetError::AccessFault)
        } else {
            Ok(())
        }
    }

    fn mem_write(&self, addr: u32, src: &[u8]) -> Result<(), TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        mem_write(self.target, addr, src)?;
        if check_error(self.target) {
            Err(TargetError::AccessFault)
        } else {
            Ok(())
        }
    }

    fn mem_read_nostop(&self, dest: &mut [u8], addr: u32) -> Result<bool, TargetError> {
        if self.needs_halt_for_mem() {
            return Ok(false);
        }
        self.mem_read(dest, addr)?;
        Ok(true)
    }

    fn mem_write_nostop(&self, addr: u32, src: &[u8]) -> Result<bool, TargetError> {
        if self.needs_halt_for_mem() {
            return Ok(false);
        }
        self.mem_write(addr, src)?;
        Ok(true)
    }

    fn needs_halt_for_mem(&self) -> bool {
        TargetView::new(self.target).options() & crate::target::TOPT_NON_HALTING_MEM_IO == 0
    }

    fn reg_read(&self, reg: u32, buf: &mut [u8]) -> Result<usize, TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        Ok(reg_read(self.target, reg, buf.as_mut_ptr().cast(), buf.len()))
    }

    fn reg_write(&self, reg: u32, data: &[u8]) -> Result<usize, TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        Ok(reg_write(self.target, reg, data.as_ptr().cast(), data.len()))
    }

    fn regs_read(&self, buf: &mut [u8]) -> Result<(), TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        regs_read_hook(self.target, buf.as_mut_ptr().cast());
        Ok(())
    }

    fn regs_write(&self, data: &[u8]) -> Result<(), TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        regs_write_hook(self.target, data.as_ptr().cast());
        Ok(())
    }

    fn regs_size(&self) -> usize {
        TargetView::new(self.target).regs_size()
    }

    fn reset(&self) {
        if !self.target.is_null() {
            // A driver that declares its own reset sequence owns the reset; `None` means it has
            // no opinion and the generic spec-level sequence runs. This is the hook the port to
            // Rust was missing for the per-part resets the C targets carried in
            // `target_s::extended_reset`.
            if let Some(driver) = crate::arm::driver_registry::active_driver_for(self.target)
                && driver
                    .custom_reset(&mut crate::target::CortexMAccess::new(self.target))
                    .is_some()
            {
                return;
            }
            reset(self.target);
        }
    }

    fn halt_request(&self) {
        if !self.target.is_null() {
            halt_request(self.target);
        }
    }

    fn halt_poll(&self) -> (HaltReason, u32) {
        if self.target.is_null() {
            return (HaltReason::Running, 0);
        }
        halt_poll(self.target)
    }

    fn halt_resume(&self, step: bool) {
        if !self.target.is_null() {
            halt_resume(self.target, step);
        }
    }

    fn attach(&mut self) -> Result<(), TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        let ok = if let Some(driver) = crate::arm::driver_registry::active_driver_for(self.target) {
            driver.attach(&mut crate::target::CortexMAccess::new(self.target))
        } else {
            attach(self.target)
        };
        if ok {
            TargetView::new(self.target).set_attached(true);
            Ok(())
        } else {
            Err(TargetError::AccessFault)
        }
    }

    fn detach(&mut self) {
        crate::bmp::disconnect_cleanup(self.target);
        if !self.target.is_null() {
            if let Some(driver) = crate::arm::driver_registry::active_driver_for(self.target) {
                driver.detach(&mut crate::target::CortexMAccess::new(self.target));
            } else {
                detach(self.target);
            }
            TargetView::new(self.target).set_attached(false);
        }
    }

    fn breakwatch_set(&mut self, breakwatch: &mut Breakwatch) -> Result<(), TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        breakwatch_set(self.target, breakwatch)
    }

    fn breakwatch_clear(&mut self, breakwatch: &Breakwatch) -> Result<(), TargetError> {
        if self.target.is_null() {
            return Err(TargetError::NotAttached);
        }
        breakwatch_clear(self.target, breakwatch)
    }

    fn breakwatch_clear_all(&mut self) {
        if !self.target.is_null() {
            breakwatch_clear_all(self.target);
        }
    }
}
