//! Interior mutability cell for single-threaded embedded contexts.
//!
//! Provides `SyncCell<T>`, wrapping an `UnsafeCell<T>` with a `Sync` implementation.
//! Safe to use in Swindle's single-threaded embedded execution model.

use core::cell::UnsafeCell;

/// Cell providing interior mutability in single-threaded environments.
pub struct SyncCell<T>(UnsafeCell<T>);

unsafe impl<T> Sync for SyncCell<T> {}

impl<T> SyncCell<T> {
    pub const fn new(val: T) -> Self {
        SyncCell(UnsafeCell::new(val))
    }

    #[inline(always)]
    pub const fn as_ptr(&self) -> *mut T {
        self.0.get()
    }

    /// # Safety
    /// Caller must ensure no mutable reference to the inner value is live.
    #[inline(always)]
    pub unsafe fn as_ref(&self) -> &T {
        unsafe { &*self.0.get() }
    }

    /// # Safety
    /// Caller must ensure no other reference to the inner value is live.
    #[inline(always)]
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn as_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
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

