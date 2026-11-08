//! Dispatch of the ARM (Cortex-M) target driver that is currently probed.
//!
//! The probe entry points (`stm32f1_probe`, `gd32f1_probe`, `stm32f4_probe`, `gd32f4_probe`,
//! `stm32g0_probe`, `rp2040_probe`, `rp2350_probe`) live in
//! `rs_swindle_wrapper/src/arm_probe_chain.rs`, because they are the only code that names the
//! concrete driver crates: those crates depend on this one, so naming them here would be a
//! dependency cycle. This module is everything *after* the probe. The shim installs the
//! driver it probed into a [`DriverSlot`] it owns and calls [`probe`]; from then on the
//! driver is reached as an [`ActiveArmDriver`], i.e. as a `dyn RustArmTargetDriver`.
//!
//! This is the ARM sibling of `crate::riscv::driver_registry`.

#![allow(unsafe_op_in_unsafe_fn)]

use crate::target::{
    ActiveDriverSlot, CortexMAccess, DriverSlot, DriverTag, RustArmTargetDriver, Target,
    TargetView, install_part,
};

/// The ARM driver of the target that is currently probed, type erased.
///
/// A driver crate cannot be named from here (see the module docs), and only one ARM target is
/// live at a time, so the shim keeps the driver in a [`DriverSlot`] and this module keeps the
/// `dyn RustArmTargetDriver` pointer to it.
pub type ActiveArmDriver = &'static mut (dyn RustArmTargetDriver + 'static);

/// The one live ARM driver and the family tag it was installed under.
static ACTIVE_DRIVER: ActiveDriverSlot<dyn RustArmTargetDriver> = ActiveDriverSlot::new();

/// The active Rust ARM driver, only if it matches `target`.
pub(crate) fn active_driver_for(target: *mut Target) -> Option<ActiveArmDriver> {
    if target.is_null() {
        return None;
    }
    // An ARM driver never describes a RISC-V hart. `part_id` is the ARM DBGMCU device id on
    // one side and a RISC-V chip id on the other, so the numeric match below would be a
    // coincidence with a driver for another architecture behind it.
    if crate::riscv::get_riscv_context(target).is_some() {
        return None;
    }
    let driver = ACTIVE_DRIVER.get_mut()?;
    if TargetView::new(target).part_id() == driver.part_id() {
        Some(driver)
    } else {
        None
    }
}

/// The active ARM driver, as the concrete driver type `D`.
///
/// This is the escape hatch for a driver's own `mon` callbacks: they are plain
/// `fn(&str, &[&str]) -> bool` C-trampolined pointers with no driver handle, so instead of
/// publishing a copy of the probe results in a file-scope `static mut`, they look the live
/// driver up here and project a temporary view out of it. `None` means the attached target
/// belongs to another driver (or none), i.e. the command does not apply.
pub fn active_arm_driver_as<D: RustArmTargetDriver + 'static>() -> Option<&'static D> {
    let driver: &'static (dyn RustArmTargetDriver + 'static) =
        unsafe { &*ACTIVE_DRIVER.get_raw()? };
    let driver: &'static dyn core::any::Any = driver;
    driver.downcast_ref::<D>()
}

/// The active ARM driver, mutably, as the concrete driver type `D`.
///
/// For the `mon` commands that *set* driver state (`mon psize` on the F4 family, `mon
/// irreversible` on the G0 family): that state belongs to the driver, so the command mutates
/// it there rather than in a file-scope static the driver has to read back.
///
/// # Safety
///
/// Only called by such a `mon` command, which never runs while one of the driver's own
/// `&mut self` hooks is on the stack (see [`active_arm_driver_as`]); holding the returned
/// borrow across a call into a flash/attach hook would alias.
pub unsafe fn active_arm_driver_as_mut<D: RustArmTargetDriver + 'static>() -> Option<&'static mut D>
{
    let driver: &'static mut (dyn RustArmTargetDriver + 'static) = ACTIVE_DRIVER.get_mut()?;
    let driver: &'static mut dyn core::any::Any = driver;
    driver.downcast_mut::<D>()
}

/// Run a driver crate's `probe()` on `target` and attach the driver it returns.
///
/// `slot` is the caller's handle for the one driver family and `tag` names that family: the
/// probe entry point owns them because only it knows the concrete type. The driver itself goes
/// into the one shared `DRIVER_STORAGE` cell.
pub unsafe fn probe<D>(
    target: *mut Target,
    slot: &'static DriverSlot<D>,
    tag: DriverTag,
    probe: fn(&mut CortexMAccess) -> Option<D>,
) -> bool
where
    D: RustArmTargetDriver + 'static,
{
    if target.is_null() {
        return false;
    }
    let mut cm = CortexMAccess::new(target);
    match probe(&mut cm) {
        Some(driver) => {
            let (tag, driver) = slot.install(tag, driver);
            attach(target, tag, driver)
        }
        None => false,
    }
}

/// Attach and configure `target` from a pure Rust ARM driver.
///
/// Installs the part description the driver reports, then publishes `driver` — under the
/// family `tag` it was installed with — as the one active driver.
///
/// Kept out of line: every ARM probe entry point (`arm_probe_chain`) calls it, and inlining it
/// there would give each caller its own copy of the same dozen instructions.
#[inline(never)]
pub unsafe fn attach(target: *mut Target, tag: DriverTag, driver: ActiveArmDriver) -> bool {
    if target.is_null() {
        return false;
    }
    install_part(target, &*driver);
    ACTIVE_DRIVER.set(tag, driver);
    true
}
