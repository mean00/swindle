//! Generic Cortex debug layer: the Rust half of `blackmagic/src/target/cortex.c`.
//!
//! This is the code every ARM debug core shares: the link from a `target_s` to the
//! ADIv5 access port it hangs off, the debug-window accessors that translate a small
//! register offset into the core's debug address space, and the CPUID decode that names
//! the core and flags the architecture flavour.
//!
//! The C file read those things out of `((cortex_priv_s *)target->priv)`, a struct only
//! the Cortex-M layer ever filled in, and followed `ap->dp` directly. Here the state is
//! [`CortexPriv`] — owned by the Rust Cortex-M layer, which keeps it as the first field of
//! its own `CortexMPriv` — and the ADIv5 internals come from `super::c_seam`, the one place
//! that declares the `bmp_helpers.cpp` calls and wraps each in a safe function.
//!
//! The `extern "C"` names the still-C parts of the tree call (`cortex_ap()`,
//! `cortex_check_error()`, `cortex_dbg_read32()`, …) are the thin adapters in
//! `super::c_api`; this module itself is plain Rust, with the one `unsafe` deref hidden in
//! [`CortexPriv::ref_of()`].

use crate::target::{Target, TargetView};

// C `DEBUG_INFO` output: compiled out, as it is in the C build.
crate::setup_log!(false);

/* --- the CPUID offsets, field masks and core part numbers live in
 * `super::cortexm_registers` --- */
use super::cortexm_registers::*;

/// `CORTEXM_TOPT_FLAVOUR_V6M`: the core is ARMv6-M (M0/M0+).
pub const CORTEXM_TOPT_FLAVOUR_V6M: u32 = 1 << 1;
/// `CORTEXM_TOPT_FLAVOUR_FLOAT`: the core has an FPU.
pub const CORTEXM_TOPT_FLAVOUR_FLOAT: u32 = 1 << 2;
/// `CORTEXM_TOPT_FLAVOUR_V8M`: the core is ARMv8-M (M23/M33/M55).
pub const CORTEXM_TOPT_FLAVOUR_V8M: u32 = 1 << 3;
/// `CORTEXM_TOPT_TRUSTZONE`: the core implements the security extension.
pub const CORTEXM_TOPT_TRUSTZONE: u32 = 1 << 4;

/// JEP-106 code of Atmel, whose protected parts report an unusual CPUID part number.
const JEP106_MANUFACTURER_ATMEL: u32 = 0x01f;

/// Logical `AccessPort`: the ADIv5 port the core hangs off.
pub use crate::arm::adi::AccessPort;

/// The state the generic Cortex layer keeps per target: C `cortex_priv_s`.
///
/// It is the first field of the Cortex-M layer's private state, which is what
/// `target->priv` points at — the `#[repr(C)]` prefix is what lets the exported
/// `cortex_*()` functions below find it from a `target_s` alone. C never casts
/// `target->priv` itself, so the rest of the layout is the Rust side's business.
#[repr(C)]
pub struct CortexPriv {
    /// Access port the core hangs off.
    pub ap: *mut AccessPort,
    /// Base address of the debug interface block (C `base_addr`, `CORTEXM_SCS_BASE`).
    pub base_addr: u32,
    /// Instruction cache line length in words (0 = no cache).
    pub icache_line_length: u16,
    /// Data cache line length in words (0 = no cache).
    pub dcache_line_length: u16,
    /// Bit per FPB slot in use.
    pub breakpoints_mask: u8,
    /// Bit per DWT slot in use.
    pub watchpoints_mask: u8,
    /// DWT comparators the core implements.
    pub watchpoints_available: u8,
    /// FPB code comparators the core implements.
    pub breakpoints_available: u8,
}

impl CortexPriv {
    /// A zeroed private state for `ap`, ready to be embedded in the Cortex-M state.
    pub fn new(ap: *mut AccessPort, base_addr: u32) -> Self {
        Self {
            ap,
            base_addr,
            icache_line_length: 0,
            dcache_line_length: 0,
            breakpoints_mask: 0,
            watchpoints_mask: 0,
            watchpoints_available: 0,
            breakpoints_available: 0,
        }
    }

    /// The private state of `target`, or null when it carries none.
    ///
    /// The pointer is only valid while that allocation lives; it is a handle owned by
    /// `target_s`, like the target handle itself, so this is not marked `unsafe`.
    pub fn of(target: *mut Target) -> *mut CortexPriv {
        TargetView::new(target).priv_ptr().cast()
    }

    /// The private state of `target`, or `None` when it carries none.
    ///
    /// The borrow is as long as the target's: the Cortex-M layer allocates the state in its
    /// `probe()` and only frees it when the target is released.
    pub fn ref_of(target: *mut Target) -> Option<&'static CortexPriv> {
        // Safety: `of()` is either null (→ `None`) or the live allocation `target_s` owns.
        unsafe { Self::of(target).as_ref() }
    }
}

use super::c_seam::*;


/// The access port of `target` (C `ap()`).
pub(crate) fn ap(target: *mut Target) -> *mut AccessPort {
    CortexPriv::ref_of(target).map_or(core::ptr::null_mut(), |priv_| priv_.ap)
}

/// True when the access port of `target` reports a sticky error (C `check_error()`).
pub(crate) fn check_error(target: *mut Target) -> bool {
    let ap = ap(target);
    !ap.is_null() && dp_error(ap) != 0
}

/// Read a word of the core's debug address space (C `dbg_read32()`).
pub(crate) fn dbg_read32(target: *mut Target, src: u16) -> u32 {
    let Some(priv_) = CortexPriv::ref_of(target) else {
        return 0;
    };
    let (ap, base_addr) = (priv_.ap, priv_.base_addr);
    if ap.is_null() {
        return 0;
    }
    let mut value = [0u8; 4];

    // The debug window is served by the AP's banked data registers, which are only
    // reachable while the DP is not in banked-AP mode.
    ap_clear_banked(ap);
    let _ = ap_mem_read(ap, &mut value, base_addr + u32::from(src));
    u32::from_le_bytes(value)
}

/// Write a word of the core's debug address space (C `dbg_write32()`).
#[allow(dead_code)]
pub(crate) fn dbg_write32(target: *mut Target, dest: u16, value: u32) {
    let Some(priv_) = CortexPriv::ref_of(target) else {
        return;
    };
    let (ap, base_addr) = (priv_.ap, priv_.base_addr);
    if ap.is_null() {
        return;
    }

    ap_clear_banked(ap);
    let _ = ap_mem_write(ap, base_addr + u32::from(dest), &value.to_le_bytes());
}

/// Core name and architecture flavour (`CORTEXM_TOPT_*` bits) for a `CPUID.PARTNO`.
fn core_for_partno(partno: u32) -> Option<(&'static str, u32)> {
    let core = match partno {
        CORTEX_A5 => ("A5", 0),
        CORTEX_A7 => ("A7", 0),
        CORTEX_A8 => ("A8", 0),
        CORTEX_A9 => ("A9", 0),
        CORTEX_A55 => ("A55", 0),
        CORTEX_R4 => ("R4", 0),
        CORTEX_R5 => ("R5", 0),
        STAR_MC1 => ("STAR-MC1", 0),
        CORTEX_M33 => ("M33", CORTEXM_TOPT_FLAVOUR_V8M),
        CORTEX_M23 => ("M23", CORTEXM_TOPT_FLAVOUR_V8M),
        CORTEX_M55 => ("M55", CORTEXM_TOPT_FLAVOUR_V8M),
        CORTEX_M3 => ("M3", 0),
        CORTEX_M4 => ("M4", 0),
        CORTEX_M7 => ("M7", 0),
        CORTEX_M0P => ("M0+", CORTEXM_TOPT_FLAVOUR_V6M),
        CORTEX_M0 => ("M0", CORTEXM_TOPT_FLAVOUR_V6M),
        _ => return None,
    };
    Some(core)
}

/// Read and decode `CPUID`, name the core, OR its flavour bits into the target's options
/// and return those bits (C `read_cpuid()`).
///
/// The bits are ORed in, so the architecture field the probe has already stored survives,
/// and returning them keeps the Cortex-M probe from reading the field back through the C
/// seam: the layers above this one are Rust too.
pub(crate) fn cpuid_flavour(target: *mut Target) -> u32 {
    let view = TargetView::new(target);
    let cpuid = dbg_read32(target, CORTEX_CPUID);
    view.set_cpuid(cpuid);

    let partno = cpuid & CORTEX_CPUID_PARTNO_MASK;
    let core = core_for_partno(partno);
    let flavour = match core {
        Some((name, flavour)) => {
            view.set_core(name);
            // OR: the caller has already marked the target's architecture field.
            view.or_options(flavour);
            // First M7 revisions step into a pending exception handler.
            if partno == CORTEX_M7
                && cpuid & CORTEX_CPUID_REVISION_MASK == 0
                && cpuid & CORTEX_CPUID_PATCH_MASK < 2
            {
                bmpwarning!("Silicon bug: Single stepping will enter pending exception handler with this M7 core revision!\n");
            }
            flavour
        }
        None => {
            // A protected Atmel part reports a part number nobody else uses; anything else
            // is worth reporting.
            let ap = ap(target);
            if !ap.is_null()
                && ap_designer_code(ap) != JEP106_MANUFACTURER_ATMEL
            {
                bmpwarning!("Unexpected Cortex CPU partno {:04x}\n", partno);
            }
            0
        }
    };

    bmplog!(
        "CPUID 0x{:08x} ({} var {} rev {})\n",
        cpuid,
        core.map(|(name, _)| name).unwrap_or(""),
        (cpuid & CORTEX_CPUID_REVISION_MASK) >> 20,
        cpuid & CORTEX_CPUID_PATCH_MASK
    );

    flavour
}

/// Read and decode `CPUID`, naming the core and setting its flavour option bits
/// (C `read_cpuid()`: a thin wrapper over [`cpuid_flavour()`] for the C ABI shim).
#[allow(dead_code)]
pub(crate) fn read_cpuid(target: *mut Target) {
    cpuid_flavour(target);
}

