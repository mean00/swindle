//! Hosted half of the RISC-V scans: the `riscv_dmi_s` this process builds for a
//! target that answers behind the RPC link.
//!
//! The probe owns the wire (the RVSWD tap, or the SDI bit-bang); the target list
//! lives here, so a hosted scan has to build its own DM transport out of the RPC
//! leaves and hand it to [`riscv_dmi_init()`] — exactly what the native
//! `rvswd_scan()` / `sdi_scan()` do with the on-probe tap.
//!
//! This module holds the shared attach machinery plus the RVSWD leg
//! (`rvswd_scan()`, the host side of `mon rvswdp_scan`). The SDI leg stays with
//! the rest of the SDI seam in `crate::riscv_extra`, which reuses
//! [`attach_dmi()`] with its own RPC callbacks.

use core::mem::MaybeUninit;

use crate::SyncCell;
use crate::riscv::dm_regs::{
    RV_DM_COMMAND, RV_DM_CONTROL, RV_DM_CTRL_ACTIVE, RV_DM_CTRL_HALT_REQ, RV_DM_DATA0, RV_DM_DATA1,
    RV_DM_STAT_VERSION_0_13, RV_DM_STATUS,
};
use crate::riscv::dmi::{DmiLeg, RiscvDmiC};
use crate::riscv::probe::riscv_dmi_init;
use crate::target::jep106;

// bmplog! is a compile-time toggle: setup_log!(false) makes it a no-op in this
// file (see bmplogger.rs).
crate::setup_log!(false);

/// `riscv_dmi_s.read` over the RPC link: address in, value out.
pub type DmiReadFn =
    unsafe extern "C" fn(dmi: *mut RiscvDmiC, address: u32, value: *mut u32) -> bool;
/// `riscv_dmi_s.write` over the RPC link: address and value in.
pub type DmiWriteFn = unsafe extern "C" fn(dmi: *mut RiscvDmiC, address: u32, value: u32) -> bool;

/// `riscv_dmi_s.fault` values (`riscv_debug.h`).
pub const RV_DMI_SUCCESS: u8 = 0;
pub const RV_DMI_FAILURE: u8 = 2;

/// WCH parts carry a 7-bit DMI address field in their frames
/// (`start + 7·addr + RW`), like `wchlink_riscv_dtm.c`.
const WCH_DMI_ADDRESS_WIDTH: u8 = 7;

/// The transport object of the current session.
///
/// `RiscvHart` stores the `CDmiTransport` that points here, so it has to outlive
/// the scan: a stack local would leave every later DM access of the attached
/// target dangling. One slot is enough — every scan starts by freeing the target
/// list, and no target outlives its own scan.
static DMI_SLOT: SyncCell<MaybeUninit<RiscvDmiC>> = SyncCell::new(MaybeUninit::uninit());

/// Which wire [`DMI_SLOT`] reaches the DM on. Set by [`attach_dmi()`] together with the
/// transport, so the two can never disagree; the bulk memory leaves send it to the probe
/// (see `bulk_mem.rs`), whose entry points differ per wire.
static DMI_LEG: SyncCell<u8> = SyncCell::new(DmiLeg::Rvswd.wire_id());

/// The wire of the current session's transport (RVSWD until a scan attached another).
pub fn current_leg() -> DmiLeg {
    DmiLeg::from_wire_id(u32::from(DMI_LEG.get())).unwrap_or(DmiLeg::Rvswd)
}

/// Clear the target list and attach a RISC-V target over an RPC-backed DM.
///
/// Mirror of the probe-side scans: the transport is described here (designer =
/// WCH, debug spec 0.13, 7-bit DMI addresses — the shape every WCH part
/// answers), then [`riscv_dmi_init()`] discovers the hart and runs the
/// per-family probe. `leg` is the wire `read` / `write` drive.
///
/// # Safety
/// Single-shot per session: the transport lives in [`DMI_SLOT`] and must not be
/// overwritten while a target that uses it is still attached.
pub unsafe fn attach_dmi(leg: DmiLeg, read: DmiReadFn, write: DmiWriteFn) -> bool {
    unsafe {
        crate::target::target_list_free();
        DMI_LEG.set(leg.wire_id());
        let dmi = (*DMI_SLOT.as_ptr()).as_mut_ptr();
        core::ptr::write_bytes(dmi, 0, 1);
        (*dmi).designer_code = jep106::WCH as u16;
        (*dmi).version = RV_DM_STAT_VERSION_0_13; /* Assumption, unverified */
        (*dmi).address_width = WCH_DMI_ADDRESS_WIDTH;
        (*dmi).read = Some(read);
        (*dmi).write = Some(write);

        riscv_dmi_init(dmi);
        !crate::target::target_list.is_null()
    }
}

// ---------------------------------------------------------------------------
// RVSWD leg
// ---------------------------------------------------------------------------

/// WCH chip-id register: answers 0x203/0x303/0x305-style ids.
const WCH_DM_CHIP_ID_REG: u32 = 0x7f;
/// What the bus reads back on `WCH_DM_CHIP_ID_REG` with no target answering.
const WCH_CHIP_ID_NO_ANSWER: u32 = 0x7fff;
/// `DM_CONTROL` write of the unlock sequence: halt request + DM active.
const WCH_DM_CTRL_UNLOCK: u32 = RV_DM_CTRL_HALT_REQ | RV_DM_CTRL_ACTIVE;
/// Unlock sequence values, reverse engineered from a WCH RVSWD capture (the
/// `bmda_rv_dm_probe()` sequence of the retired hosted C glue).
const WCH_DM_DATA1_UNLOCK: u32 = 0x1fff_f704;
const WCH_DM_COMMAND_UNLOCK: u32 = 0x0220_0000;

/// Bounded retry budget for one DMI access, like the probe-side DMI layer
/// (`RVSWD_DMI_MAX_ATTEMPTS`): the reply carries the probe's own DM status word,
/// so a "busy" frame is retried instead of being reported as a failure.
const RVSWD_DMI_MAX_ATTEMPTS: u32 = 10;

/// One DM access over the RPC link, result ignored.
///
/// Used by the unlock sequence, whose steps are unconditional in the reference
/// implementation: a missing answer only shows up in the chip-id read.
fn rvswd_dm_access(address: u32, value: Option<u32>, out: &mut u32) {
    unsafe {
        crate::hosted::c_stubs::ln_rv_tx_count = crate::hosted::c_stubs::ln_rv_tx_count.wrapping_add(1);
    }
    match value {
        Some(v) => {
            let _ = crate::hosted::rpc_host_generated::remote_rv_dm_write(address, v);
        }
        None => {
            let (ok, data) = crate::hosted::rpc_host_generated::remote_rv_dm_read(address);
            if ok {
                *out = data;
            }
        }
    }
}

/// `riscv_dmi_s.read` over the RV RPC leg.
unsafe extern "C" fn rvswd_dmi_read(dmi: *mut RiscvDmiC, address: u32, value: *mut u32) -> bool {
    if dmi.is_null() || value.is_null() {
        return false;
    }
    for _ in 0..RVSWD_DMI_MAX_ATTEMPTS {
        unsafe {
            crate::hosted::c_stubs::ln_rv_tx_count = crate::hosted::c_stubs::ln_rv_tx_count.wrapping_add(1);
        }
        let (ok, data) = crate::hosted::rpc_host_generated::remote_rv_dm_read(address);
        if ok {
            unsafe {
                *value = data;
                (*dmi).fault = RV_DMI_SUCCESS;
            }
            return true;
        }
    }
    unsafe {
        (*dmi).fault = RV_DMI_FAILURE;
    }
    false
}

/// `riscv_dmi_s.write` over the RV RPC leg.
unsafe extern "C" fn rvswd_dmi_write(dmi: *mut RiscvDmiC, address: u32, value: u32) -> bool {
    if dmi.is_null() {
        return false;
    }
    for _ in 0..RVSWD_DMI_MAX_ATTEMPTS {
        unsafe {
            crate::hosted::c_stubs::ln_rv_tx_count = crate::hosted::c_stubs::ln_rv_tx_count.wrapping_add(1);
        }
        let (ok, _) = crate::hosted::rpc_host_generated::remote_rv_dm_write(address, value);
        if ok {
            unsafe {
                (*dmi).fault = RV_DMI_SUCCESS;
            }
            return true;
        }
    }
    unsafe {
        (*dmi).fault = RV_DMI_FAILURE;
    }
    false
}

/// Bring the WCH DM up and read the chip id.
///
/// `None` when the sequence does not produce a plausible id (nothing on the
/// wire, or a DM that will not answer) — the gate `bmda_rv_dm_probe()` used.
fn rvswd_dm_probe() -> Option<u32> {
    let mut value: u32 = 0;
    rvswd_dm_access(0x38u32, Some(0x5aa55aa5), &mut value);
    // Two control writes select the hart and start the DM.
    rvswd_dm_access(
        u32::from(RV_DM_CONTROL),
        Some(WCH_DM_CTRL_UNLOCK),
        &mut value,
    );
    rvswd_dm_access(
        u32::from(RV_DM_CONTROL),
        Some(WCH_DM_CTRL_UNLOCK),
        &mut value,
    );
    rvswd_dm_access(u32::from(RV_DM_STATUS), None, &mut value);
    rvswd_dm_access(WCH_DM_CHIP_ID_REG, None, &mut value);
    let chip_id = value;
    rvswd_dm_access(
        u32::from(RV_DM_DATA1),
        Some(WCH_DM_DATA1_UNLOCK),
        &mut value,
    );
    rvswd_dm_access(
        u32::from(RV_DM_COMMAND),
        Some(WCH_DM_COMMAND_UNLOCK),
        &mut value,
    );
    rvswd_dm_access(u32::from(RV_DM_DATA0), None, &mut value);
    rvswd_dm_access(u32::from(RV_DM_DATA1), None, &mut value);

    if (chip_id & WCH_CHIP_ID_NO_ANSWER) != WCH_CHIP_ID_NO_ANSWER {
        Some(chip_id)
    } else {
        None
    }
}

/// Hosted `mon rvswdp_scan`: probe a WCH RVSWD target over the RPC link and
/// attach it as a RISC-V target.
///
/// Host counterpart of the native `rvswdp_scan()` (`swindle/src/template/
/// rvswd_template.h`) and of the retired `bmda_rvswd_scan2()`.
#[unsafe(no_mangle)]
pub extern "C" fn rvswdp_scan() -> bool {
    // `rv_dm_start()` on the probe: RVSWD pin mux, wait-state floor, DM reset.
    let (ok, _) = crate::hosted::rpc_host_generated::remote_rv_reset();
    if !ok {
        bmpwarning!("RVSWD : the probe did not answer the reset\n");
        return false;
    }

    let chip_id = match rvswd_dm_probe() {
        Some(id) => id,
        None => {
            bmpwarning!("RVSWD : no WCH device found\n");
            return false;
        }
    };
    bmplog!("RVSWD : found 0x{:x} device\n", chip_id);

    unsafe { attach_dmi(DmiLeg::Rvswd, rvswd_dmi_read, rvswd_dmi_write) }
}
