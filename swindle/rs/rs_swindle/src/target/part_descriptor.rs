//! The architecture-neutral half of the driver contract, shared by both driver traits.
//!
//! A driver *describes the part* it probed: its name, part id, target options, memory map and
//! its own `mon` commands. None of that depends on how the probe reaches the core, so it lives
//! here, and both [`RustArmTargetDriver`](super::arm_driver::RustArmTargetDriver) and
//! [`RustRiscvTargetDriver`](super::riscv_driver::RustRiscvTargetDriver) have this trait as a
//! supertrait. The halves that *do* depend on the transport (flash session and CRC32, plus
//! breakpoints) stay in the architecture traits, which take a `&mut CortexMAccess` or a
//! `&mut RiscvHart`.
//!
//! Keeping the description in one trait is what stops it from drifting apart: with a copy per
//! architecture, the memory map alone had two different shapes (`MemoryMap` by value on ARM,
//! lent-out `&[RamDesc]` fields on RISC-V) that had to be kept in step by hand.
//!
//! [`install_part`] is the one place that turns a description into the C
//! `target_s`: both dispatchers (`crate::arm::driver_registry` and
//! `crate::riscv::driver_registry`) call it, so a name, an option flag or a memory map is
//! installed identically on both architectures. What differs between them — which callbacks
//! the transport can answer for — comes in as a [`PartHooks`] table, so a dispatcher states it
//! as data instead of replaying a hand-written sequence of setters.

use super::{Target, TargetView};
use super::memory_map::MemoryMap;
use crate::commands::mon::register_driver_commands;
use crate::commands::CommandTree;

/// What every driver says about its part, whichever architecture the core is.
pub trait PartDescriptor {
    /// Human-readable driver / chip family name, used verbatim as the C
    /// `target_s::driver` (so it usually ends with a NUL) and as the key the driver's
    /// `mon` commands are registered under.
    fn name(&self) -> &'static str;

    /// Chip part ID (e.g. the raw DBGMCU IDCODE for the STM32 families).
    fn part_id(&self) -> u32;

    /// Target options flags (`TOPT_*`), e.g.
    /// [`TOPT_INHIBIT_NRST`](super::arm_driver::TOPT_INHIBIT_NRST).
    fn target_options(&self) -> u32 {
        0
    }

    /// Whether the core has a usable hardware trigger unit.
    ///
    /// A property of the part, so it is described here even though only one transport asks:
    /// `install_part` sets `target_s::no_hw_breakpoint` from this. Default: yes, which is
    /// every Cortex-M and every hart with a trigger unit.
    fn has_hw_breakpoints(&self) -> bool {
        true
    }

    /// Memory map to register with the debugger (flash + RAM regions).
    ///
    /// Computed from what the probe found and returned by value ([`MemoryMap`]), so a driver
    /// does not have to keep the regions in its own storage.
    fn memory_map(&self) -> MemoryMap;

    /// Extra `mon` (GDB `monitor`) commands this driver exposes.
    ///
    /// They are registered under [`Self::name`] once the driver probes, and installed as soon
    /// as the attached target's driver name (`bmp_get_target_name()`, i.e. the C
    /// `target_s::driver`, NUL-stripped) starts with that name. The default is an empty set:
    /// only the generic `mon` commands are available.
    fn commands(&self) -> &'static [CommandTree] {
        &[]
    }
}

/// Install `part`'s description into the C `target_s` `target`.
///
/// Sets the fields the driver reports (name, part id, options, hardware breakpoint flag)
/// and registers the driver's own `mon` commands. Both dispatchers call this and then
/// publish the driver as the active one.
///
/// # Safety
///
/// `target` must be a live C `target_s` the caller owns until it is freed.
pub unsafe fn install_part(target: *mut Target, part: &dyn PartDescriptor) {
    if !target.is_null() {
        let view = TargetView::new(target);
        view.set_driver_str(part.name());
        view.set_part_id(part.part_id());
        view.or_options(part.target_options());
        view.set_no_hw_breakpoint(!part.has_hw_breakpoints());

        // Driver-declared `mon` commands, stored under the driver name so that
        // `mon::add_target_commands()` installs them when this driver's target is attached.
        register_driver_commands(part.name(), part.commands());
    }
}
