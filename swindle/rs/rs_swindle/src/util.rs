#![allow(dead_code)]

use alloc::alloc::Layout;
use alloc::alloc::alloc;
use cty::size_t;

/*
 *
 */
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

//-----------
pub fn unsafe_slice_alloc<T>(count: usize) -> &'static mut [T] {
    let itm = core::mem::size_of::<T>();
    unsafe {
        let ptr = malloc(itm * count);
        core::slice::from_raw_parts_mut(ptr as *mut T, count)
    }
}

pub fn unsafe_array_alloc<T>(count: usize) -> *mut T {
    let itm = core::mem::size_of::<T>();
    unsafe {
        let ptr = malloc(itm * count);
        ptr as *mut T
    }
}

pub fn unsafe_box_allocate<T>() -> *mut T {
    let layout = Layout::new::<T>();
    unsafe { alloc(layout) as *mut T }
}
