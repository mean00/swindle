//! Dispatch of the RISC-V target driver that is currently probed.
//!
//! The probe entry point (`riscv_rust_target_probe`, called from `riscv_dmi_init`) lives in
//! `rs_swindle_wrapper/src/riscv_shim.rs`, because it is the only code that names the
//! concrete driver crates: those crates depend on this one, so naming them here would be a
//! dependency cycle. This module is everything *after* a probe. The shim installs the driver
//! it probed into a [`DriverSlot`] it owns and calls [`probe`]; from then on the driver is
//! reached as an [`ActiveRiscvDriver`], i.e. as a `dyn RustRiscvTargetDriver`.
//!
//! This is the RISC-V sibling of `crate::arm::driver_registry`.

#![allow(unsafe_op_in_unsafe_fn)]

use crate::riscv::{RiscvHart, get_riscv_hart};
use crate::target::{
    ActiveDriverSlot, DriverSlot, DriverTag, RustRiscvTargetDriver, Target, install_part,
};

/// The RISC-V driver of the target that is currently probed, type erased.
///
/// A driver crate cannot be named from here (see the module docs), and only one target is
/// live at a time, so the shim keeps the driver in a [`DriverSlot`] and this module keeps the
/// `dyn RustRiscvTargetDriver` pointer to it.
pub type ActiveRiscvDriver = &'static mut (dyn RustRiscvTargetDriver + 'static);

/// The one live RISC-V driver and the family tag it was installed under.
static ACTIVE_DRIVER: ActiveDriverSlot<dyn RustRiscvTargetDriver> = ActiveDriverSlot::new();

/// The driver of the currently probed RISC-V target, if a probe installed one.
pub(crate) fn active_driver() -> Option<ActiveRiscvDriver> {
    ACTIVE_DRIVER.get_mut()
}

/// The active RISC-V driver, as the concrete driver type `D`.
///
/// The RISC-V sibling of [`crate::arm::active_arm_driver_as`]: a driver's `mon` callbacks get
/// no handle, so they resolve the live driver here and project a temporary view out of it
/// instead of publishing a copy of the probe results in a file-scope `static mut`. `None`
/// means the attached target belongs to another driver (or none).
pub fn active_riscv_driver_as<D: RustRiscvTargetDriver + 'static>() -> Option<&'static D> {
    let driver: &'static (dyn RustRiscvTargetDriver + 'static) =
        unsafe { &*ACTIVE_DRIVER.get_raw()? };
    let driver: &'static dyn core::any::Any = driver;
    driver.downcast_ref::<D>()
}

/// The active RISC-V driver, mutably, as the concrete driver type `D`.
///
/// The mutable sibling of [`active_riscv_driver_as`], for a `mon` command that *sets* driver
/// state. `unsafe` for the same reason as its ARM counterpart: the caller must not hold the
/// borrow across a call into one of the driver's own hooks.
pub unsafe fn active_riscv_driver_as_mut<D: RustRiscvTargetDriver + 'static>()
-> Option<&'static mut D> {
    let driver: &'static mut (dyn RustRiscvTargetDriver + 'static) = ACTIVE_DRIVER.get_mut()?;
    let driver: &'static mut dyn core::any::Any = driver;
    driver.downcast_mut::<D>()
}

/// Number of execute-breakpoint slots the active driver reports for its trigger unit.
///
/// The Rust side of `mon breakpoint_info`: the C target has no trigger-unit count for a
/// Rust-driven RISC-V target, so the count comes from the driver that probed the hart.
pub fn active_hw_breakpoint_count() -> u32 {
    active_driver().map_or(0, |driver| driver.hw_breakpoint_count())
}

/// Run a driver crate's `probe()` on `target` and attach the driver it returns.
///
/// `slot` is the caller's handle for the one driver family and `tag` names that family: the
/// probe entry point owns them because only it knows the concrete type. The driver itself goes
/// into the one shared `DRIVER_STORAGE` cell. The probe order across families is the caller's
/// business, so a probe entry point chains one call per driver crate.
pub unsafe fn probe<D>(
    target: *mut Target,
    slot: &'static DriverSlot<D>,
    tag: DriverTag,
    probe: fn(&mut RiscvHart) -> Option<D>,
) -> bool
where
    D: RustRiscvTargetDriver + 'static,
{
    let Some(hart) = get_riscv_hart(target) else {
        return false;
    };
    match probe(hart) {
        Some(driver) => {
            let (tag, driver) = slot.install(tag, driver);
            attach(target, tag, driver)
        }
        None => false,
    }
}

/// Attach and configure `target` from a pure Rust RISC-V driver.
///
/// Installs the part description the driver reports, then publishes `driver` — under the
/// family `tag` it was installed with — as the one active driver.
///
/// Kept out of line: every RISC-V probe entry point (`riscv_shim`) calls it, and inlining it
/// there would give each caller its own copy of the same dozen instructions.
#[inline(never)]
pub unsafe fn attach(target: *mut Target, tag: DriverTag, driver: ActiveRiscvDriver) -> bool {
    if target.is_null() {
        return false;
    }
    install_part(target, &*driver);
    ACTIVE_DRIVER.set(tag, driver);
    true
}
