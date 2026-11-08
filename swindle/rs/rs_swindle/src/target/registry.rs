//! Target registry managing the shared driver storage cell and active driver slots.

use crate::sync_cell::SyncCell;
use core::cell::UnsafeCell;
use core::marker::PhantomData;

/// Size of the shared driver cell, in bytes.
///
/// One cell holds the driver of whichever family probed last, so it has to fit the largest of
/// them: 0x60 B today (CH32V00x), checked by `install` at every probe. 256 B leaves room for
/// several more families without moving the `.bss` boundary again.
pub const DRIVER_STORAGE_SIZE: usize = 256;

/// Alignment of the shared driver cell: a driver may hold `u64` fields (Cortex-M privs do).
pub const DRIVER_STORAGE_ALIGN: usize = 8;

/// Which family's driver the shared cell holds.
///
/// A name instead of a number, so a mismatch reported by a `mon` command is readable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DriverTag(pub &'static str);

/// The live driver instance of the firmware, whatever its family or architecture.
///
/// Only one target is attached at a time, so only one driver is ever live — and driver
/// instances are big (48…96 B each) while most of them are never used. A `static` per family,
/// as they used to be, therefore paid RAM for every family the firmware can probe; one shared
/// cell pays for the largest only.
///
/// The probe entry points (`arm_probe_chain.rs`, `riscv_shim.rs` in `rs_swindle_wrapper`) are
/// the only code that names the concrete driver crates, so they are the only code that can
/// [`install`](DriverStorage::install) into the cell. What they hand out — a `&'static mut D`
/// — is what the architecture dispatchers (`arm::driver_registry`, `riscv::driver_registry`)
/// keep behind `dyn RustArmTargetDriver` / `dyn RustRiscvTargetDriver`, together with the
/// [`DriverTag`] they installed under: that tag is how a dispatcher notices that a later probe
/// of another family has overwritten its driver.
#[repr(align(8))]
pub struct DriverStorage {
    tag: UnsafeCell<Option<DriverTag>>,
    bytes: UnsafeCell<[u8; DRIVER_STORAGE_SIZE]>,
}

// SAFETY: the cell is written on the probe path and read back through the dispatchers'
// active-driver pointers; both are reached from the single-threaded GDB dispatch.
unsafe impl Sync for DriverStorage {}

impl DriverStorage {
    /// Cold cell: no driver installed, bytes unused.
    pub const fn new() -> Self {
        Self {
            tag: UnsafeCell::new(None),
            bytes: UnsafeCell::new([0; DRIVER_STORAGE_SIZE]),
        }
    }

    /// Store `driver` and return the tag it was stored under with the reference the dispatcher
    /// keeps.
    ///
    /// The tag comes back because the dispatcher publishes the pair as its own active-driver
    /// cell: handing it back means it never has to read the shared cell to learn what it just
    /// wrote.
    ///
    /// Installing a second driver overwrites the previous one without handing it out again,
    /// which is what a re-probe wants: only one target is live at a time.
    ///
    /// # Safety
    ///
    /// `self` must be a `static`, so the returned reference can live for the rest of the
    /// session. The caller must not keep using a reference handed out by an earlier install —
    /// that holds across families and across architectures, since the cell is shared.
    pub unsafe fn install<D>(
        &'static self,
        tag: DriverTag,
        driver: D,
    ) -> (DriverTag, &'static mut D) {
        // A driver is a plain value: it is overwritten in place, never dropped.
        assert!(
            !core::mem::needs_drop::<D>(),
            "a driver must not need drop: the shared cell overwrites it in place"
        );
        assert!(
            core::mem::size_of::<D>() <= DRIVER_STORAGE_SIZE,
            "driver does not fit the shared cell"
        );
        assert!(
            core::mem::align_of::<D>() <= DRIVER_STORAGE_ALIGN,
            "driver is over-aligned for the shared cell"
        );

        // SAFETY: the caller guarantees `self` is a `static` and that no earlier install's
        // reference is still in use, so this is the only live borrow of the cell.
        let slot: *mut D = self.bytes.get().cast::<D>();
        unsafe { slot.write(driver) };
        unsafe { *self.tag.get() = Some(tag) };
        (tag, unsafe { &mut *slot })
    }

    /// Family whose driver the cell holds, `None` before the first install.
    pub fn tag(&self) -> Option<DriverTag> {
        // SAFETY: `DriverTag` is `Copy` and the cell is only written from the probe path.
        unsafe { *self.tag.get() }
    }
}

impl Default for DriverStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// The one live driver instance of the firmware.
pub static DRIVER_STORAGE: DriverStorage = DriverStorage::new();

/// Handle of one driver family, zero-sized: the storage is [`DRIVER_STORAGE`].
///
/// A `static` of this type is what a probe entry point names per family; it costs no RAM at
/// all, so a family that is compiled in but never probed costs nothing. `PhantomData<fn() ->
/// D>` keeps the handle `Sync` without claiming to own a `D`.
pub struct DriverSlot<D>(PhantomData<fn() -> D>);

impl<D> DriverSlot<D> {
    /// Handle to the shared cell, for a `static` per driver family.
    pub const fn new() -> Self {
        Self(PhantomData)
    }

    /// Store `driver` in the shared cell under `tag`, and return the tag with the reference the
    /// dispatcher keeps. See [`DriverStorage::install`] for the safety contract.
    pub unsafe fn install(&'static self, tag: DriverTag, driver: D) -> (DriverTag, &'static mut D) {
        unsafe { DRIVER_STORAGE.install(tag, driver) }
    }
}

impl<D> Default for DriverSlot<D> {
    fn default() -> Self {
        Self::new()
    }
}

/// Architecture-level active driver pointer paired with the [`DriverTag`] it was installed under.
pub struct ActiveDriverSlot<D: ?Sized> {
    inner: SyncCell<Option<(DriverTag, *mut D)>>,
}

impl<D: ?Sized + 'static> ActiveDriverSlot<D> {
    pub const fn new() -> Self {
        Self {
            inner: SyncCell::new(None),
        }
    }

    pub fn set(&self, tag: DriverTag, driver: *mut D) {
        unsafe { *self.inner.as_ptr() = Some((tag, driver)) };
    }

    pub fn get_raw(&self) -> Option<*mut D> {
        let (tag, driver) = unsafe { (*self.inner.as_ptr())? };
        (DRIVER_STORAGE.tag() == Some(tag)).then_some(driver)
    }

    pub fn get_mut(&self) -> Option<&'static mut D> {
        Some(unsafe { &mut *self.get_raw()? })
    }
}

impl<D: ?Sized + 'static> Default for ActiveDriverSlot<D> {
    fn default() -> Self {
        Self::new()
    }
}

