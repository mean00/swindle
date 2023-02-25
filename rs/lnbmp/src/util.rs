use alloc::alloc::Layout as Layout;
use alloc::alloc::alloc as alloc;
use cty::size_t;

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
