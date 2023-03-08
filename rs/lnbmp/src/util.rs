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

//--
pub fn hex_to_u8s<'a,'b>(sin : &'a str, sout: &'b mut [u8]) -> Result<&'b [u8],i32>
{
    let datain = sin.as_bytes();

    let s= datain.len()/2;
    for i in 0..s
    {
        sout[i]=ascii_to_hex( datain[i*2],datain[i*2+1]);
    }
    return Ok(&sout[..s]);
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
//---
pub fn ascii_to_hex( left : u8, right : u8 ) -> u8
{

    (_hex(left)<<4)+_hex(right)
}
//---
pub fn _tohex( v: u8) -> u8
{
    if v>=10
    {
        return b'A' +v -10;
    }
    return b'0'+v;
}
//---
pub fn ascii_to_u32(s : &str) -> u32
{
    let datain = s.as_bytes();
    let mut val  : u32 = 0;
    for i in 0..datain.len()
    {
        val=(val<<4)+_hex(datain[i]) as u32;
    }
    val
}
pub fn u8_to_ascii( value : u8 ) -> [u8;2]
{
    let mut out : [u8;2 ]= [0,0];
    out[0]=_tohex(value>>4);
    out[1]=_tohex(value&0xf);
    out
    
}
pub fn u8_to_ascii_to_buffer( value : u8 , out : &mut [u8])
{
    out[0]=_tohex(value>>4);
    out[1]=_tohex(value&0xf);
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

pub fn take_adress_length( xin : &str ) -> Option< (u32, u32) >
{
    let args : Vec <&str>= xin.split(",").collect();
    if args.len()!=2
    {
        glog("take_adress_length : wrong param");
        return None;
    }
    let address = crate::util::ascii_to_u32(args[0]);
    let len = crate::util::ascii_to_u32(args[1]);
    Some ( (address,len) )
}

pub fn  split_command<'a>( incoming : &'a [u8]) -> Option< (&'a [u8], &'a [u8]) >
{
    let size =incoming.len();
    // look up for the first ':' if any
    // and split there
    for i in 0..size
    {
        if incoming[i] == b':'
        {
            // split here
            if i==size-1
            {
                return Some( (&incoming[..i],&incoming[0..0]) );
            }
            return Some( (&incoming[..i],&incoming[(i+1)..] ) );            
        }
    }
    return Some( (incoming,&incoming[0..0]));
}