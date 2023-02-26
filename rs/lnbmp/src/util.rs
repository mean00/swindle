use alloc::alloc::Layout as Layout;
use alloc::alloc::alloc as alloc;
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
#[cfg(feature = "native")]
use rnarduino::rn_os_helper::log;
#[cfg(feature = "native")]
pub fn glog (s : &str)
{
    log(s);
}

/**
 * 
 */
fn _hex( digit : u8 ) -> u8
{
    return match digit
    {
        b'0'..=b'9' =>  digit -b'0',
        b'a'..=b'f' =>  digit +10 -b'a',
        b'A'..=b'F' =>  digit +10 -b'A',
        _ => 0, // WTF ?
    }
}
pub fn ascii_to_hex( left : u8, right : u8 ) -> u8
{

    (_hex(left)<<4)+_hex(right)
}

pub fn xabs(x: isize) -> isize
{
    if x < 0         {return -x;}
    x
}
pub fn xswap( a: &mut isize, b : &mut isize)
{
    let z: isize = *a;
    *a=*b;
    *b=z;
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
