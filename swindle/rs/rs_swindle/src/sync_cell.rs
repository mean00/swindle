use core::cell::UnsafeCell;

/// A wrapper around `UnsafeCell` that implements `Sync`.
///
/// This is safe only in single-threaded embedded contexts where the
/// `OnceLock` + `UnsafeCell` pattern is used for lazy-initialized mutable
/// statics. The caller must ensure that access is serialized (e.g., via
/// `OnceLock`'s initialization guarantee and single-threaded execution).
#[repr(transparent)]
pub struct SyncUnsafeCell<T: ?Sized> {
    inner: UnsafeCell<T>,
}

// SAFETY: This type is only used in single-threaded embedded debugger contexts.
// The `OnceLock` ensures initialization happens exactly once, and all subsequent
// access is from the same thread (the C main loop).
unsafe impl<T: ?Sized + Send> Sync for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    /// Creates a new `SyncUnsafeCell` containing the given value.
    #[inline]
    pub const fn new(val: T) -> Self {
        SyncUnsafeCell {
            inner: UnsafeCell::new(val),
        }
    }

    /// Gets a mutable pointer to the contained value.
    #[inline]
    pub fn get(&self) -> *mut T {
        self.inner.get()
    }
}