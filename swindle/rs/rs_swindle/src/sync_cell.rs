//! Interior mutability cell for single-threaded embedded contexts.
//!
//! Provides `SyncCell<T>`, wrapping an `UnsafeCell<T>` with a `Sync` implementation.
//! Safe to use in Swindle's single-threaded embedded execution model.

use core::cell::UnsafeCell;

/// Cell providing interior mutability for Copy types in single-threaded environments.
pub struct SyncCell<T>(UnsafeCell<T>);

unsafe impl<T> Sync for SyncCell<T> {}

impl<T> SyncCell<T> {
    pub const fn new(val: T) -> Self {
        SyncCell(UnsafeCell::new(val))
    }

    pub fn get(&self) -> T
    where
        T: Copy,
    {
        unsafe { *self.0.get() }
    }

    pub fn set(&self, val: T)
    where
        T: Copy,
    {
        unsafe {
            *self.0.get() = val;
        }
    }
}
