//! RISC-V target probe entry point and target context lifecycle management.
#![allow(unsafe_op_in_unsafe_fn)]

use super::dm_regs::*;
use super::dmi::{CDmiTransport, DmiTransport, RiscvDmiC};
use super::hart::{HartIsa, RiscvHart};
use crate::sync_cell::SyncCell;
use crate::target::{Target, TargetView, target_new};
use core::ffi::c_void;
use core::mem::MaybeUninit;

/// Target context held in `target->priv`.
pub struct RiscvTargetContext {
    pub hart: RiscvHart,
}

// ---------------------------------------------------------------------------
// Context storage: a `.bss` pool instead of `Box` (malloc_used.md §0.8, C9)
// ---------------------------------------------------------------------------

/// How many `RiscvTargetContext` slots to reserve.
///
/// Every scan frees the previous context first — `rvswd_scan()` / `sdi_scan()`
/// call `target_list_free()`, which runs `priv_free` before `riscv_dmi_init()`
/// allocates — so exactly one slot is live at a time. The second slot is
/// spare, so that a context that somehow never gets freed cannot make the next
/// scan fail outright.
const CTX_SLOT_COUNT: usize = 2;

/// Storage for the live contexts; `CTX_SLOT_USED[i]` says which slots are taken.
static CTX_SLOTS: SyncCell<[MaybeUninit<RiscvTargetContext>; CTX_SLOT_COUNT]> =
    SyncCell::new([const { MaybeUninit::uninit() }; CTX_SLOT_COUNT]);
static CTX_SLOT_USED: SyncCell<[bool; CTX_SLOT_COUNT]> =
    SyncCell::new([false; CTX_SLOT_COUNT]);

/// Move `value` into a free slot and hand out a pointer to it.
///
/// Returns `None` when all slots are taken; the caller must then report that no
/// target could be created rather than fall back to the heap.
fn ctx_alloc(value: RiscvTargetContext) -> Option<*mut RiscvTargetContext> {
    unsafe {
        let slots = CTX_SLOTS.as_mut();
        let used = CTX_SLOT_USED.as_mut();
        for (slot, taken) in slots.iter_mut().zip(used.iter_mut()) {
            if !*taken {
                *taken = true;
                return Some(slot.write(value));
            }
        }
        None
    }
}

/// Give a slot handed out by [`ctx_alloc`] back to the pool.
///
/// Returns `false` (and does nothing) if `ptr` is not one of the pool's slots,
/// so a context that did not come from here can never be released twice.
///
/// # Safety
/// `ptr` must be null or a pointer returned by [`ctx_alloc`] whose slot has not
/// been released yet.
unsafe fn ctx_free(ptr: *mut RiscvTargetContext) -> bool {
    unsafe {
        let slots = CTX_SLOTS.as_mut();
        let used = CTX_SLOT_USED.as_mut();
        for (slot, taken) in slots.iter_mut().zip(used.iter_mut()) {
            if core::ptr::eq(slot.as_ptr(), ptr) {
                if !*taken {
                    return false;
                }
                // `RiscvTargetContext` holds no `Drop` state today, but dropping
                // keeps the slot reusable if that ever changes.
                slot.assume_init_drop();
                *slot = MaybeUninit::uninit();
                *taken = false;
                return true;
            }
        }
        false
    }
}

#[allow(improper_ctypes)]
unsafe extern "C" {
    /// Probe entry of the pure Rust RISC-V drivers (CH32V00x / CH32V1xx-3xx / RP2350 Hazard3).
    ///
    /// It lives in `rs_swindle_wrapper/src/riscv_shim.rs` because that is the only crate
    /// that names the driver crates, which depend on this one. The shim also owns the probe
    /// order between the families, which is why nothing else is called from here.
    fn riscv_rust_target_probe(target: *mut Target) -> bool;
}

// ---------------------------------------------------------------------------
// Target lifecycle and probe entry point
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn riscv_dmi_init(dmi: *mut RiscvDmiC) {
    if dmi.is_null() {
        return;
    }
    let mut transport = CDmiTransport::new(dmi);

    let dmstatus = match transport.read(RV_DM_STATUS) {
        Ok(s) => s,
        Err(_) => return,
    };
    let version = dmstatus & RV_DM_STAT_VERSION_MASK;
    if version != RV_DM_STAT_VERSION_0_13 && version != RV_DM_STAT_VERSION_1_0 {
        return;
    }

    let hart = match RiscvHart::probe(transport, 0) {
        Ok(h) => h,
        Err(_) => return,
    };

    let ctx_ptr = match ctx_alloc(RiscvTargetContext { hart }) {
        Some(ptr) => ptr,
        // All slots taken: no target this time (never falls back to the heap).
        None => return,
    };

    let target = target_new();
    if target.is_null() {
        ctx_free(ctx_ptr);
        return;
    }

    let isa = (*ctx_ptr).hart.isa;
    let core_name = match isa {
        HartIsa::Rv32e => "rv32e",
        HartIsa::Rv32i => "rv32i",
        HartIsa::Rv32iSingleFpu => "rv32if",
    };

    let view = TargetView::new(target);
    view.set_priv(ctx_ptr);
    view.set_priv_free(riscv_target_priv_free);
    view.set_driver("RISC-V");
    view.set_core(core_name);
    view.set_designer_code(u32::from((*dmi).designer_code));
    view.set_regs_size(isa.regs_size());
    view.set_arch_riscv();

    let _ = riscv32_probe(target);

    let ctx_ref = &mut *ctx_ptr;
    // The accessor and its speed were settled in `RiscvHart::probe()` (the driver
    // probe above needs memory access already); this only reports the choice.
    super::benchmark::report_selected_method(&ctx_ref.hart);

    // Resume target after probing completes
    let _ = ctx_ref.hart.halt_resume(false);
}

pub unsafe fn riscv32_probe(target: *mut Target) -> bool {
    if target.is_null() {
        false
    } else {
        riscv_rust_target_probe(target)
    }
}

/// Borrow the `RiscvTargetContext` that `target`'s `priv` points at.
///
/// Returns `None` when there is no target or it carries no context.
///
/// The lifetime is unbound for the same reason as in [`get_riscv_context`]: the
/// context lives in `CTX_SLOTS` and never moves while the target is listed.
fn ctx_of<'a>(target: *mut Target) -> Option<&'a mut RiscvTargetContext> {
    let priv_ptr = TargetView::new(target).priv_ptr();
    if priv_ptr.is_null() {
        None
    } else {
        // SAFETY: `priv` is only ever set to a slot of `CTX_SLOTS` by `riscv_dmi_init()`.
        unsafe { Some(&mut *(priv_ptr as *mut RiscvTargetContext)) }
    }
}

/// Helper to extract `RiscvTargetContext` from a `Target` pointer if it is an attached RISC-V target.
pub fn get_riscv_context<'a>(target: *mut Target) -> Option<&'a mut RiscvTargetContext> {
    if target.is_null() {
        return None;
    }
    let opts = TargetView::new(target).options();
    if ((opts >> 28) & 0x03) != 2 {
        return None;
    }
    ctx_of(target)
}

/// Helper to extract `&mut RiscvHart` from a `Target` pointer if it is an attached RISC-V target.
pub fn get_riscv_hart<'a>(target: *mut Target) -> Option<&'a mut super::hart::RiscvHart> {
    get_riscv_context(target).map(|ctx| &mut ctx.hart)
}

/// Helper to get the `RiscvTargetContext` for the currently selected target.
pub fn current_riscv_context<'a>() -> Option<&'a mut RiscvTargetContext> {
    let cur = crate::target::get_cur_target();
    get_riscv_context(cur)
}

/// Helper to get the `&mut RiscvHart` for the currently selected target.
pub fn current_riscv_hart<'a>() -> Option<&'a mut super::hart::RiscvHart> {
    current_riscv_context().map(|ctx| &mut ctx.hart)
}

unsafe fn riscv_target_priv_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        // Returns the pool slot (malloc_used.md §0.8, C9) instead of freeing a
        // heap block; a pointer that is not ours is left alone.
        ctx_free(ptr as *mut RiscvTargetContext);
    }
}
