use alloc::alloc::Layout as Layout;
use alloc::alloc::alloc as alloc;
use alloc::vec::Vec;
use cty::size_t;


/**
 * 
 */
#[cfg(feature = "hosted")]
extern crate std;
#[cfg(feature = "hosted")]
use std::print;
#[cfg(feature = "hosted")]
pub fn glog (s : &str)
{
    print!("<<{:?}\n",s);
}
#[cfg(feature = "hosted")]
pub fn glog1<T: core::fmt::Display> (s : &str, v: T)
{
    print!("<<{:?}:{}\n",s,v);
}
#[cfg(feature = "hosted")]
pub fn glogx (s : &str, v: u32)
{
    print!("<<{:?}:0x{:#x}\n",s,v);
}

#[cfg(feature = "native")]
use rnarduino::rn_os_helper::{log,log1};

#[cfg(feature = "native")]
pub fn glog1<T: uDisplay> (s : &str, v: T)
{
    log1(s,v);
}

use ufmt::uDisplay;
#[cfg(feature = "native")]
pub fn glog (s : &str)
{
    log(s);
}
#[cfg(feature = "native")]
pub fn glogx (s : &str, v: u32)
{
    log1(s,v);
}

pub fn xswap( a: &mut isize, b : &mut isize)
{
    let z: isize = *a;
    *a = *b;
    *b = z;
}
pub fn xmin(a : isize, b: isize) -> isize
{
    if a< b { return a;}
    b
}
pub fn xmax(a : isize, b: isize) -> isize
{
    if b< a { return a;}
    b
}

pub fn xminu(a : usize, b: usize) -> usize
{
    if a< b { return a;}
    b
}
pub fn xmaxu(a : usize, b: usize) -> usize
{
    if b< a { return a;}
    b
}
//
//https://stackoverflow.com/questions/59232877/how-to-allocate-structs-on-the-heap-without-taking-up-space-on-the-stack-in-stab

//-----------
extern "C"
{
pub fn malloc(size: size_t) -> *mut cty::c_void;
}

//-----------
pub fn unsafe_slice_alloc<T>(count : usize ) -> &'static mut[T]
{    
        let itm = core::mem::size_of::<T>();
        unsafe {
                let   ptr = malloc(itm*count);
                core::slice::from_raw_parts_mut(ptr as *mut T,count)
        }

}

pub fn unsafe_array_alloc<T>(count : usize ) -> *mut T 
{    
        let itm = core::mem::size_of::<T>();
        unsafe {
                let   ptr = malloc(itm*count);
                ptr as *mut T
        }

}

pub fn unsafe_box_allocate<T>() ->  *mut T
{
    
    let layout = Layout::new::<T>();
    unsafe {           
        let ptr = alloc(layout) as *mut T;             
        ptr
    }
}
