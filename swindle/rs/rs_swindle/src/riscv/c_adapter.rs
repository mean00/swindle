//! C-ABI shims adapting RISC-V debug subsystem to C layers.
#![allow(unsafe_op_in_unsafe_fn)]

use super::dm::*;
use super::dmi::{CDmiTransport, DmiTransport, RiscvDmiC};
use super::hart::{HartIsa, RiscvHart};
use super::mem::RiscvMem;
use crate::target::c_adapter::{target_new, TargetC};
use crate::target::driver::HaltReason;
use core::ffi::c_void;
use core::mem::MaybeUninit;

/// C-compatible mirror of the `riscv_hart_s` header layout.
#[repr(C)]
pub struct RiscvHartC {
    pub dbg_module: *mut c_void,
    pub hart_idx: u32,
    pub hartsel: u32,
}

/// Target context held in `target->priv`.
pub struct RiscvTargetContext {
    pub c_header: RiscvHartC,
    pub hart: RiscvHart,
}

// ---------------------------------------------------------------------------
// Context storage: a `.bss` pool instead of `Box` (malloc_used.md §0.8, C9)
// ---------------------------------------------------------------------------

/// How many `RiscvTargetContext` slots to reserve.
///
/// Every scan frees the previous context first — `rvswd_scan()` / `sdi_scan()`
/// call `target_list_free()`, which runs `priv_free` before `riscv_dmi_init()`
/// allocates — so exactly one slot is live at a time.  The second slot is
/// spare, so that a context that somehow never gets freed cannot make the next
/// scan fail outright.
const CTX_SLOT_COUNT: usize = 2;

/// Storage for the live contexts; `CTX_SLOT_USED[i]` says which slots are taken.
static mut CTX_SLOTS: [MaybeUninit<RiscvTargetContext>; CTX_SLOT_COUNT] =
    [const { MaybeUninit::uninit() }; CTX_SLOT_COUNT];
static mut CTX_SLOT_USED: [bool; CTX_SLOT_COUNT] = [false; CTX_SLOT_COUNT];

/// Move `value` into a free slot and hand out a pointer to it.
///
/// Returns `None` when all slots are taken; the caller must then report that no
/// target could be created rather than fall back to the heap.
fn ctx_alloc(value: RiscvTargetContext) -> Option<*mut RiscvTargetContext> {
    unsafe {
        let slots = &mut *(&raw mut CTX_SLOTS);
        let used = &mut *(&raw mut CTX_SLOT_USED);
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
        let slots = &mut *(&raw mut CTX_SLOTS);
        let used = &mut *(&raw mut CTX_SLOT_USED);
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

#[repr(C)]
pub struct TargetRiscvOpsC {
    pub priv_ptr: *mut c_void,
    pub priv_free: Option<unsafe extern "C" fn(ptr: *mut c_void)>,
    pub driver: *const core::ffi::c_char,
    pub core: *const core::ffi::c_char,
    pub designer_code: u16,
    pub regs_size: usize,
    pub attach: Option<unsafe extern "C" fn(t: *mut TargetC) -> bool>,
    pub detach: Option<unsafe extern "C" fn(t: *mut TargetC)>,
    pub check_error: Option<unsafe extern "C" fn(t: *mut TargetC) -> bool>,
    pub mem_read: Option<unsafe extern "C" fn(t: *mut TargetC, dest: *mut c_void, src: u64, len: usize)>,
    pub mem_write: Option<unsafe extern "C" fn(t: *mut TargetC, dest: u64, src: *const c_void, len: usize)>,
    pub mem_read_nostop: Option<unsafe extern "C" fn(t: *mut TargetC, dest: *mut c_void, src: u64, len: usize) -> bool>,
    pub mem_write_nostop: Option<unsafe extern "C" fn(t: *mut TargetC, dest: u64, src: *const c_void, len: usize) -> bool>,
    pub reg_read: Option<unsafe extern "C" fn(t: *mut TargetC, reg: u32, data: *mut c_void, max: usize) -> usize>,
    pub reg_write: Option<unsafe extern "C" fn(t: *mut TargetC, reg: u32, data: *const c_void, size: usize) -> usize>,
    pub regs_read: Option<unsafe extern "C" fn(t: *mut TargetC, data: *mut c_void)>,
    pub regs_write: Option<unsafe extern "C" fn(t: *mut TargetC, data: *const c_void)>,
    pub reset: Option<unsafe extern "C" fn(t: *mut TargetC)>,
    pub halt_request: Option<unsafe extern "C" fn(t: *mut TargetC)>,
    pub halt_resume: Option<unsafe extern "C" fn(t: *mut TargetC, step: bool)>,
    pub halt_poll: Option<unsafe extern "C" fn(t: *mut TargetC, watch: *mut u64) -> u32>,
    pub breakwatch_set: Option<unsafe extern "C" fn(t: *mut TargetC, bw: *mut c_void) -> i32>,
    pub breakwatch_clear: Option<unsafe extern "C" fn(t: *mut TargetC, bw: *mut c_void) -> i32>,
}

unsafe extern "C" {
    fn target_get_priv_c(t: *mut TargetC) -> *mut c_void;
    fn target_set_riscv_ops_c(t: *mut TargetC, ops: *const TargetRiscvOpsC);
    fn riscv32_silicon_probe_c(target: *mut TargetC) -> bool;
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

    let ctx_ptr = match ctx_alloc(RiscvTargetContext {
        c_header: RiscvHartC {
            dbg_module: core::ptr::null_mut(),
            hart_idx: hart.hart_idx,
            hartsel: hart.hartsel,
        },
        hart,
    }) {
        Some(ptr) => ptr,
        // All slots taken: no target this time (never falls back to the heap).
        None => return,
    };
    // The C-visible header points back at its own context, which never moves
    // again: it lives in `CTX_SLOTS` for the rest of the scan.
    (*ctx_ptr).c_header.dbg_module = ctx_ptr as *mut c_void;

    let target = target_new();
    if target.is_null() {
        ctx_free(ctx_ptr);
        return;
    }

    let priv_ptr = ctx_ptr as *mut c_void;

    let core_name = match (*(priv_ptr as *mut RiscvTargetContext)).hart.isa {
        HartIsa::Rv32e => b"rv32e\0".as_ptr() as *const core::ffi::c_char,
        HartIsa::Rv32i => b"rv32i\0".as_ptr() as *const core::ffi::c_char,
        HartIsa::Rv32iSingleFpu => b"rv32if\0".as_ptr() as *const core::ffi::c_char,
    };

    let designer = (*dmi).designer_code;
    let regs_size = (*(priv_ptr as *mut RiscvTargetContext)).hart.isa.regs_size();

    let ops = TargetRiscvOpsC {
        priv_ptr,
        priv_free: Some(riscv_target_priv_free),
        driver: b"RISC-V\0".as_ptr() as *const core::ffi::c_char,
        core: core_name,
        designer_code: designer,
        regs_size,
        attach: Some(riscv_target_attach),
        detach: Some(riscv_target_detach),
        check_error: Some(riscv_target_check_error),
        mem_read: Some(riscv_target_mem_read),
        mem_write: Some(riscv_target_mem_write),
        mem_read_nostop: Some(riscv_target_mem_read_nostop),
        mem_write_nostop: Some(riscv_target_mem_write_nostop),
        reg_read: Some(riscv_target_reg_read),
        reg_write: Some(riscv_target_reg_write),
        regs_read: Some(riscv_target_regs_read),
        regs_write: Some(riscv_target_regs_write),
        reset: Some(riscv_target_reset),
        halt_request: Some(riscv_target_halt_request),
        halt_resume: Some(riscv_target_halt_resume),
        halt_poll: Some(riscv_target_halt_poll),
        breakwatch_set: Some(riscv_target_breakwatch_set),
        breakwatch_clear: Some(riscv_target_breakwatch_clear),
    };

    target_set_riscv_ops_c(target, &ops);

    let _ = riscv32_probe(target);

    // Automatically benchmark candidate memory access methods and pick the fastest
    let ctx_ref = &mut *(priv_ptr as *mut RiscvTargetContext);
    super::benchmark::auto_select_fastest_method(&mut ctx_ref.hart);

    // Resume target after probing completes
    let _ = ctx_ref.hart.halt_resume(false);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn riscv32_probe(target: *mut TargetC) -> bool {
    if target.is_null() {
        false
    } else {
        riscv32_silicon_probe_c(target)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn riscv_attach(target: *mut TargetC) -> bool {
    riscv_target_attach(target)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn riscv_detach(target: *mut TargetC) {
    riscv_target_detach(target);
}

/// Helper to extract `RiscvTargetContext` from a `TargetC` pointer if it is an attached RISC-V target.
pub fn get_riscv_context<'a>(target: *mut TargetC) -> Option<&'a mut RiscvTargetContext> {
    if target.is_null() {
        return None;
    }
    let opts = unsafe { crate::target::c_adapter::target_get_options_c(target) };
    if ((opts >> 28) & 0x03) != 2 {
        return None;
    }
    let priv_ptr = unsafe { target_get_priv_c(target) };
    if priv_ptr.is_null() {
        None
    } else {
        Some(unsafe { &mut *(priv_ptr as *mut RiscvTargetContext) })
    }
}

/// Helper to extract `&mut RiscvHart` from a `TargetC` pointer if it is an attached RISC-V target.
pub fn get_riscv_hart<'a>(target: *mut TargetC) -> Option<&'a mut super::hart::RiscvHart> {
    get_riscv_context(target).map(|ctx| &mut ctx.hart)
}

/// Helper to get the `RiscvTargetContext` for the currently selected target.
pub fn current_riscv_context<'a>() -> Option<&'a mut RiscvTargetContext> {
    let cur = crate::target::c_adapter::get_cur_target();
    get_riscv_context(cur)
}

/// Helper to get the `&mut RiscvHart` for the currently selected target.
pub fn current_riscv_hart<'a>() -> Option<&'a mut super::hart::RiscvHart> {
    current_riscv_context().map(|ctx| &mut ctx.hart)
}

// ---------------------------------------------------------------------------
// Target method implementations
// ---------------------------------------------------------------------------

unsafe extern "C" fn riscv_target_priv_free(ptr: *mut c_void) {
    if !ptr.is_null() {
        // Returns the pool slot (malloc_used.md §0.8, C9) instead of freeing a
        // heap block; a pointer that is not ours is left alone.
        ctx_free(ptr as *mut RiscvTargetContext);
    }
}

unsafe extern "C" fn riscv_target_attach(target: *mut TargetC) -> bool {
    let priv_ptr = target_get_priv_c(target);
    if priv_ptr.is_null() {
        return false;
    }
    let ctx = &mut *(priv_ptr as *mut RiscvTargetContext);
    ctx.hart.halt_request().is_ok()
}

unsafe extern "C" fn riscv_target_detach(target: *mut TargetC) {
    crate::bmp::disconnect_cleanup(target);
}

unsafe extern "C" fn riscv_target_check_error(target: *mut TargetC) -> bool {
    let priv_ptr = target_get_priv_c(target);
    if priv_ptr.is_null() {
        return true;
    }
    let ctx = &mut *(priv_ptr as *mut RiscvTargetContext);
    let cs = ctx.hart.dmi.read(RV_DM_ABSTRACTCS).unwrap_or(0);
    (cs & RV_DM_ABSTCS_CMDERR_MASK) != 0
}

unsafe extern "C" fn riscv_target_mem_read(target: *mut TargetC, dest: *mut c_void, src: u64, len: usize) {
    let priv_ptr = target_get_priv_c(target);
    if priv_ptr.is_null() || dest.is_null() || len == 0 {
        return;
    }
    let ctx = &mut *(priv_ptr as *mut RiscvTargetContext);
    let slice = core::slice::from_raw_parts_mut(dest as *mut u8, len);
    if let Err(_e) = RiscvMem::read(&mut ctx.hart, src as u32, slice) {
        crate::glue::gdb_out_rs("RiscvMem::read failed at addr=");
        crate::bmplogger::G::print_hex(src as u32);
        crate::glue::gdb_out_rs("\n");
    }
}

unsafe extern "C" fn riscv_target_mem_write(target: *mut TargetC, dest: u64, src: *const c_void, len: usize) {
    let priv_ptr = target_get_priv_c(target);
    if priv_ptr.is_null() || src.is_null() || len == 0 {
        return;
    }
    let ctx = &mut *(priv_ptr as *mut RiscvTargetContext);
    let slice = core::slice::from_raw_parts(src as *const u8, len);
    if let Err(_e) = RiscvMem::write(&mut ctx.hart, dest as u32, slice) {
        crate::glue::gdb_out_rs("RiscvMem::write failed at addr=");
        crate::bmplogger::G::print_hex(dest as u32);
        crate::glue::gdb_out_rs("\n");
    }
}

unsafe extern "C" fn riscv_target_mem_read_nostop(
    target: *mut TargetC,
    dest: *mut c_void,
    src: u64,
    len: usize,
) -> bool {
    let priv_ptr = target_get_priv_c(target);
    if priv_ptr.is_null() || dest.is_null() || len == 0 {
        return false;
    }
    let ctx = &mut *(priv_ptr as *mut RiscvTargetContext);
    let slice = core::slice::from_raw_parts_mut(dest as *mut u8, len);
    RiscvMem::read_nostop(&mut ctx.hart, src as u32, slice).unwrap_or(false)
}

unsafe extern "C" fn riscv_target_mem_write_nostop(
    target: *mut TargetC,
    dest: u64,
    src: *const c_void,
    len: usize,
) -> bool {
    let priv_ptr = target_get_priv_c(target);
    if priv_ptr.is_null() || src.is_null() || len == 0 {
        return false;
    }
    let ctx = &mut *(priv_ptr as *mut RiscvTargetContext);
    let slice = core::slice::from_raw_parts(src as *const u8, len);
    RiscvMem::write_nostop(&mut ctx.hart, dest as u32, slice).unwrap_or(false)
}

unsafe extern "C" fn riscv_target_reg_read(
    target: *mut TargetC,
    reg: u32,
    data: *mut c_void,
    max: usize,
) -> usize {
    let priv_ptr = target_get_priv_c(target);
    if priv_ptr.is_null() || data.is_null() || max < 4 {
        return 0;
    }
    let ctx = &mut *(priv_ptr as *mut RiscvTargetContext);
    let slice = core::slice::from_raw_parts_mut(data as *mut u8, max);
    ctx.hart.reg_read(reg, slice).unwrap_or(0)
}

unsafe extern "C" fn riscv_target_reg_write(
    target: *mut TargetC,
    reg: u32,
    data: *const c_void,
    size: usize,
) -> usize {
    let priv_ptr = target_get_priv_c(target);
    if priv_ptr.is_null() || data.is_null() || size < 4 {
        return 0;
    }
    let ctx = &mut *(priv_ptr as *mut RiscvTargetContext);
    let slice = core::slice::from_raw_parts(data as *const u8, size);
    ctx.hart.reg_write(reg, slice).unwrap_or(0)
}

unsafe extern "C" fn riscv_target_regs_read(target: *mut TargetC, data: *mut c_void) {
    let priv_ptr = target_get_priv_c(target);
    if priv_ptr.is_null() || data.is_null() {
        return;
    }
    let ctx = &mut *(priv_ptr as *mut RiscvTargetContext);
    let total_size = ctx.hart.isa.regs_size();
    let slice = core::slice::from_raw_parts_mut(data as *mut u8, total_size);
    let _ = ctx.hart.regs_read(slice);
}

unsafe extern "C" fn riscv_target_regs_write(target: *mut TargetC, data: *const c_void) {
    let priv_ptr = target_get_priv_c(target);
    if priv_ptr.is_null() || data.is_null() {
        return;
    }
    let ctx = &mut *(priv_ptr as *mut RiscvTargetContext);
    let total_size = ctx.hart.isa.regs_size();
    let slice = core::slice::from_raw_parts(data as *const u8, total_size);
    let _ = ctx.hart.regs_write(slice);
}

unsafe extern "C" fn riscv_target_reset(target: *mut TargetC) {
    let priv_ptr = target_get_priv_c(target);
    if priv_ptr.is_null() {
        return;
    }
    let ctx = &mut *(priv_ptr as *mut RiscvTargetContext);
    let _ = ctx.hart.reset();
}

unsafe extern "C" fn riscv_target_halt_request(target: *mut TargetC) {
    let priv_ptr = target_get_priv_c(target);
    if priv_ptr.is_null() {
        return;
    }
    let ctx = &mut *(priv_ptr as *mut RiscvTargetContext);
    let _ = ctx.hart.halt_request();
}

unsafe extern "C" fn riscv_target_halt_resume(target: *mut TargetC, step: bool) {
    let priv_ptr = target_get_priv_c(target);
    if priv_ptr.is_null() {
        return;
    }
    let ctx = &mut *(priv_ptr as *mut RiscvTargetContext);
    let _ = ctx.hart.halt_resume(step);
}

unsafe extern "C" fn riscv_target_halt_poll(target: *mut TargetC, watch: *mut u64) -> u32 {
    let priv_ptr = target_get_priv_c(target);
    if priv_ptr.is_null() {
        return HaltReason::Error as u32;
    }
    let ctx = &mut *(priv_ptr as *mut RiscvTargetContext);
    let (reason, addr) = ctx.hart.halt_poll();
    if !watch.is_null() {
        *watch = addr as u64;
    }
    reason as u32
}

unsafe extern "C" fn riscv_target_breakwatch_set(_target: *mut TargetC, _bw: *mut c_void) -> i32 {
    // Hardware triggers: fallback -1 (software breakpoints handled via sw_breakpoints.rs)
    -1
}

unsafe extern "C" fn riscv_target_breakwatch_clear(_target: *mut TargetC, _bw: *mut c_void) -> i32 {
    -1
}

