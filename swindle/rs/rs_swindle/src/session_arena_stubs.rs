//! Stub half of the session arena seam (`SWINDLE_SESSION_ARENA=OFF`).
//!
//! `swindle/src/bmp_helpers.cpp` names the `swindle_session_*` functions
//! explicitly and the RVSWD/SDI templates call
//! `swindle_session_transport_alloc()`, so the five symbols have to exist in
//! every build; with the option off they forward straight to the heap and the
//! lifecycle hook does nothing.  The firmware then behaves exactly as before (no
//! `.bss` regions, no extra work), and `mon arena` reports that the arena is not
//! built in - the same dual-build discipline as the SDI seam
//! (`riscv_extra_sdi_stubs.rs`).

use core::ffi::c_void;
use core::ptr;

use crate::session_arena_size::{POOL_SIZE, TRANSPORT_SIZE};

/// Whether this build serves session allocations from the arena.
pub const BUILT_IN: bool = false;

unsafe extern "C" {
    /// libc `calloc`, the stub's way of saying "zeroed block": the contract of
    /// `session_arena.h` (every block comes back ALIGN-aligned and zero filled)
    /// has to hold here as well, or a `calloc` site would see garbage in an
    /// arena-off build and zeros in an arena-on one.
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// Snapshot of the arena state, for `mon arena`.  Empty here - there is no
/// region - but the sizes are the ones an arena build would have.
#[derive(Clone, Copy)]
pub struct ArenaStats {
    pub pool_used: usize,
    pub pool_size: usize,
    pub pool_peak: usize,
    pub transport_used: usize,
    pub transport_size: usize,
    pub transport_peak: usize,
    /// `free()` calls that hit arena memory.
    pub frees: u32,
    /// How many of those rewound their region.
    pub rewinds: u32,
    pub armed: bool,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn swindle_session_malloc(size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut();
    }
    unsafe { calloc(1, size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn swindle_session_calloc(count: usize, size: usize) -> *mut c_void {
    match count.checked_mul(size) {
        Some(0) | None => ptr::null_mut(),
        Some(total) => unsafe { calloc(1, total) },
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn swindle_session_free(p: *mut c_void) {
    if p.is_null() {
        return;
    }
    unsafe { free(p) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn swindle_session_transport_alloc(size: usize) -> *mut c_void {
    unsafe { swindle_session_malloc(size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn swindle_session_open() {}

/// Current arena state, for `mon arena`.
pub fn stats() -> ArenaStats {
    ArenaStats {
        pool_used: 0,
        pool_size: POOL_SIZE,
        pool_peak: 0,
        transport_used: 0,
        transport_size: TRANSPORT_SIZE,
        transport_peak: 0,
        frees: 0,
        rewinds: 0,
        armed: false,
    }
}

