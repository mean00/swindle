//! Pure Rust driver interface for RISC-V targets.
//!
//! This trait carries the half of a driver's contract that depends on the transport: the flash
//! session and erase/write/CRC32 hooks, the software-breakpoint page helpers and the hardware
//! breakpoint capability. Everything a driver says about the *part* — name, part id, target
//! options, memory map, its own `mon` commands — is in the architecture-neutral
//! [`PartDescriptor`] supertrait, shared with
//! [`super::arm_driver::RustArmTargetDriver`] (see `target/part_descriptor.rs`), so the two
//! architectures cannot drift apart in what they report.
//!
//! The traits stay distinct below that because this one is bound to `&mut RiscvHart` (RISC-V
//! DM/hart access), while an ARM driver works on a [`crate::target::CortexMAccess`] handle.
//! Keeping them apart means neither has to grow a "which architecture am I" branch and a RISC-V
//! driver cannot accidentally be reached through the Cortex-M code paths.
//!
//! Drivers implementing this trait have NO knowledge of C structs, raw pointers, or C FFI
//! callbacks: the dispatcher in this crate ([`crate::riscv::driver_registry`],
//! `ActiveRiscvDriver`) translates this interface to/from BlackMagic's C target model — first
//! [`install_part`] for the description, then its own callbacks. The probe entry point that
//! names the driver crates lives above both, in `rs_swindle_wrapper`'s `riscv_shim.rs`.

use super::part_descriptor::PartDescriptor;
use crate::flash::FlashOperation;
use crate::riscv::RiscvHart;

/// Pure Rust target driver interface for RISC-V targets.
///
/// The [`core::any::Any`] supertrait is the RISC-V half of the same mechanism the ARM trait
/// uses: a driver's `mon` callbacks get no handle, so they recover the live driver at its
/// concrete type through [`crate::riscv::active_riscv_driver_as`] and project what they need
/// out of it rather than keeping a copy in a file-scope struct.
pub trait RustRiscvTargetDriver: PartDescriptor + core::any::Any {
    /// Part-specific reset sequence, used *instead of* the generic [`RiscvHart::reset`] when a
    /// driver provides one.
    ///
    /// `Some(ok)` means "this driver reset the part, and this is whether it worked": the
    /// generic reset is then skipped. `None` - the default - means the generic sequence runs.
    ///
    /// This is where a per-part reset belongs. `riscv/hart.rs::reset()` is the spec-level
    /// sequence: assert ndmreset with haltreq, wait for `dmstatus.allhavereset`, release with
    /// haltreq held, acknowledge, invalidate caches, arm `dcsr.ebreakm`. A part that needs
    /// more than that (the legacy C drivers had `ch32v003_reset()` for the CH32V0xx, and the
    /// port to Rust had nowhere to keep it) overrides this.
    fn custom_reset(&mut self, _hart: &mut RiscvHart) -> Option<bool> {
        None
    }

    /// Number of execute-breakpoint slots the core's trigger unit offers (0 if unknown).
    ///
    /// Purely informational: it is what `mon breakpoint_info` reports for a RISC-V
    /// target, and what a later hardware-breakpoint implementation allocates from.
    fn hw_breakpoint_count(&self) -> u32 {
        0
    }

    /// Begin (or continue) a flash session (e.g. take the controller over, park the in-SRAM
    /// flashstub).
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
    fn flash_prepare(&mut self, _hart: &mut RiscvHart, _op: FlashOperation) -> bool {
        true
    }

    /// End a flash session (e.g. restore clocks/wait states, release the engine).
    ///
    /// Called once per session, never after a `flash_prepare()` that returned `false`.
    fn flash_done(&mut self, _hart: &mut RiscvHart) -> bool {
        true
    }

    /// Erase the block holding `addr`: the C manager calls this once per block.
    ///
    /// A driver that took a controller over reaches it through its session engine, which is
    /// absent outside `flash_prepare()`..`flash_done()`.
    fn flash_erase(&mut self, hart: &mut RiscvHart, addr: u32, len: usize) -> bool;

    /// Write an aligned block of data to flash.
    fn flash_write(&mut self, hart: &mut RiscvHart, dest: u32, data: &[u8]) -> bool;

    /// CRC32 (the GDB `qCRC` variant) of `[addr, addr + len)`, computed on the target.
    ///
    /// This is the Rust side of the C `target_s::crc32` hook: returning `Some` lets the
    /// debugger skip reading the whole range over the wire. `None` means "cannot do this
    /// range" (unsupported, misaligned, too short, ...) and the caller falls back to the
    /// software CRC.
    fn crc32(&mut self, _hart: &mut RiscvHart, _addr: u32, _len: u32) -> Option<u32> {
        None
    }

    /// Page size used for software breakpoint patching.
    fn sw_page_size(&self) -> u32 {
        0
    }

    /// Erase a single page for software breakpoint insertion.
    fn sw_page_erase(&mut self, _hart: &mut RiscvHart, _addr: u32) -> bool {
        false
    }

    /// Write a single page for software breakpoint insertion.
    fn sw_page_write(&mut self, _hart: &mut RiscvHart, _addr: u32, _data: &[u8]) -> bool {
        false
    }
}
