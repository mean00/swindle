//! Pure Rust driver interface for ARM (Cortex-M) targets.
//!
//! This trait carries the half of a driver's contract that depends on the transport: the flash
//! session and erase/write/CRC32 hooks, plus attach/detach and mass erase. Everything a driver
//! says about the *part* — name, part id, target options, memory map, its own `mon` commands —
//! is in the architecture-neutral [`PartDescriptor`] supertrait, which this one shares with
//! [`super::riscv_driver::RustRiscvTargetDriver`] (see `target/part_descriptor.rs`). That keeps
//! the two architectures from drifting apart: the memory map, for instance, is one
//! [`MemoryMap`](super::memory_map::MemoryMap) built at attach time, not a copy the driver keeps.
//!
//! The two traits stay distinct below that because the handle differs: this one works on a
//! [`CortexMAccess`] (Cortex-M memory access through the access port, plus
//! `cortexm_attach()`/`cortexm_detach()`), while the RISC-V one is bound to `&mut RiscvHart`
//! (RISC-V DM/hart access). Keeping them apart means neither has to grow a "which architecture
//! am I" branch and an ARM driver cannot accidentally be reached through RISC-V code paths.
//!
//! The dispatcher in this crate ([`crate::arm::driver_registry`], `ActiveArmDriver`) turns the
//! two halves into the C `target_s`: [`install_part`] for the description, then this module's
//! callbacks for the rest. The probe entry points that name the driver crates live above both,
//! in `rs_swindle_wrapper`'s `arm_probe_chain.rs`.

use super::cortexm_access::CortexMAccess;
use super::part_descriptor::PartDescriptor;
use crate::flash::FlashOperation;

/// `TOPT_INHIBIT_NRST` (`target_internal.h`): the part misbehaves when the reset line is
/// pulsed, so the probe must not use it (RP2040 boot ROM among others).
pub const TOPT_INHIBIT_NRST: u32 = 1 << 0;

/// `TOPT_NON_HALTING_MEM_IO` (`target_internal.h`): the core can be left running for
/// memory I/O, so the probe does not have to halt it around flash/CRC work.
pub const TOPT_NON_HALTING_MEM_IO: u32 = 1 << 30;

/// Pure Rust driver interface for Cortex-M targets.
///
/// The [`core::any::Any`] supertrait is what lets a driver's own `mon` callbacks find it
/// again: those get no handle (the C command trampoline passes only the packet text), so
/// they ask [`crate::arm::active_arm_driver_as`] for the live driver at this concrete type
/// and project what they need out of it, instead of keeping a copy of the probe results in
/// a second, file-scope struct.
pub trait RustArmTargetDriver: PartDescriptor + core::any::Any {
    /// Begin (or continue) a flash session: take the flash controller over, before the first
    /// erase/write.
    ///
    /// `op` is the operation about to run. One call spans a whole burst of
    /// `flash_erase`/`flash_write` calls (the manager only re-prepares when the pending
    /// operation changes), so a driver whose controller needs a save/restore area keeps it in a
    /// `session` field of its own for that long: a small typestate *engine* created here and
    /// consumed in [`Self::flash_done`]. The driver itself then only describes the part.
    /// `false` aborts the session and no `flash_done()` follows.
    ///
    /// A driver that parks flashstub images in target RAM tracks that in a
    /// [`crate::flash::StubState`] and uploads only when [`crate::flash::StubState::needs_upload`]
    /// says so, so repeated prepares inside a session cost nothing.
    fn flash_prepare(&mut self, _cm: &mut CortexMAccess, _op: FlashOperation) -> bool {
        true
    }

    /// End a flash session: give the controller back (C `flash_done`).
    ///
    /// Releases the engine [`Self::flash_prepare`] took, if any. Called once per session,
    /// never after a `flash_prepare()` that returned `false`.
    fn flash_done(&mut self, _cm: &mut CortexMAccess) -> bool {
        true
    }

    /// Erase the block holding `addr`: the C manager calls this once per block.
    ///
    /// A driver that took a controller over reaches it through its session engine, which is
    /// absent outside `flash_prepare()`..`flash_done()`.
    fn flash_erase(&mut self, cm: &mut CortexMAccess, addr: u32, len: usize) -> bool;

    /// Write an aligned block of data to flash.
    fn flash_write(&mut self, cm: &mut CortexMAccess, dest: u32, data: &[u8]) -> bool;

    /// Mass erase all flash banks (`mon erase_mass`); `false` leaves the generic
    /// region-by-region erase in charge.
    fn mass_erase(&mut self, _cm: &mut CortexMAccess) -> bool {
        false
    }

    /// Hardware CRC32 of `[addr, addr + len)` in bytes, the fast path GDB's `qCRC` packet (and
    /// so `compare-sections`) takes when the part has a CRC unit to stream it through.
    /// `None` means "no fast path here": the range then goes through the generic
    /// debug-link CRC.
    fn crc32(&mut self, _cm: &mut CortexMAccess, _addr: u32, _len: u32) -> Option<u32> {
        None
    }

    /// Part-specific reset sequence, used *instead of* the generic
    /// [`CortexMAccess::reset`] when a driver provides one.
    ///
    /// `Some(ok)` means "this driver reset the part, and this is whether it worked": the
    /// generic sequence is then skipped. `None` - the default - means the generic sequence
    /// runs.
    ///
    /// This is where a per-part reset belongs. `arm/cortexm_halt.rs::reset()` is the spec-level
    /// sequence: pulse nRST unless [`TOPT_INHIBIT_NRST`] is set, fall back to
    /// `AIRCR.SYSRESETREQ`, wait for `DHCSR.S_RESET_ST` to clear, then clear `DFSR`. A part
    /// that needs more than that overrides this: the C targets carried it in the
    /// `target_s::extended_reset` hook, whose function-pointer slot the Rust port dropped - the
    /// `TargetView::extended_reset()` stub that stood in for it was dead and has been removed,
    /// and this hook is what took its place. Calling
    /// [`CortexMAccess::reset`] from here runs the generic sequence, so a driver can wrap it
    /// with its own extra steps rather than reimplement it.
    fn custom_reset(&mut self, _cm: &mut CortexMAccess) -> Option<bool> {
        None
    }

    /// Attach hook: the driver's own peripheral setup plus the Cortex-M attach.
    /// Defaults to a plain `cortexm_attach()`.
    fn attach(&mut self, cm: &mut CortexMAccess) -> bool {
        cm.cortexm_attach()
    }

    /// Detach hook: undo [`Self::attach`]'s peripheral setup, then detach.
    fn detach(&mut self, cm: &mut CortexMAccess) {
        cm.cortexm_detach();
    }
}
