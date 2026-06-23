//! Memory allocation utilities for no_std environments.
//!
//! Provides unsafe heap allocation helpers used when the standard library's
//! `Box` or `Vec` are insufficient (e.g. for large or dynamically-sized
//! allocations in interrupt context).

#![allow(dead_code)]

use alloc::alloc::Layout;
use alloc::alloc::alloc;
use cty::size_t;

#[cfg(feature = "hosted")]
extern crate std;
#[cfg(feature = "hosted")]
#[allow(unused_imports)]
use std::print;
//
//https://stackoverflow.com/questions/59232877/how-to-allocate-structs-on-the-heap-without-taking-up-space-on-the-stack-in-stab

//-----------
unsafe extern "C" {
    pub fn malloc(size: size_t) -> *mut cty::c_void;
}

/// Allocate a `&'static mut [T]` on the heap using C `malloc`.
///
/// # Safety
///
/// The caller must ensure the returned slice is used in a single-threaded
/// context and that `T` is `Copy` or properly initialised.
pub fn unsafe_slice_alloc<T>(count: usize) -> &'static mut [T] {
    let itm = core::mem::size_of::<T>();
    unsafe {
        let ptr = malloc(itm * count);
        core::slice::from_raw_parts_mut(ptr as *mut T, count)
    }
}

/// Allocate a `*mut T` array on the heap using C `malloc`.
///
/// # Safety
///
/// The caller must ensure the returned pointer is properly sized, aligned,
/// and freed.
pub fn unsafe_array_alloc<T>(count: usize) -> *mut T {
    let itm = core::mem::size_of::<T>();
    unsafe {
        let ptr = malloc(itm * count);
        ptr as *mut T
    }
}

/// Allocate space for a single `T` on the heap using Rust's `alloc`.
///
/// # Safety
///
/// The caller must initialise the returned memory before use and ensure
/// it is properly deallocated.
pub fn unsafe_box_allocate<T>() -> *mut T {
    let layout = Layout::new::<T>();
    unsafe { alloc(layout) as *mut T }
}
