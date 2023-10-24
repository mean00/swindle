use alloc::alloc::Layout as Layout;
use alloc::alloc::alloc as alloc;
use alloc::vec::Vec;
use cty::size_t;

crate::setup_log!(false);
use crate::{bmplog,bmpwarning};
//
//
//
pub fn ascii_hex_string_to_u8s<'a,'b>(sin : &'a str, sout: &'b mut [u8]) -> Result<&'b [u8],i32>
{
    let datain = sin.as_bytes();

    let s= datain.len()/2;
    for i in 0..s
    {
        sout[i]=ascii_octet_to_hex( datain[i*2],datain[i*2+1]);
    }
    return Ok(&sout[..s]);
}
//
//
pub fn u8_hex_string_to_u8s<'a,'b>(sin : &'a [u8], sout: &'b mut [u8]) -> &'b [u8]
{
    let s= sin.len()/2;
    for i in 0..s
    {
        sout[i]=ascii_octet_to_hex( sin[i*2],sin[i*2+1]);
    }
    &sout[..s]
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
pub fn ascii_octet_to_hex( left : u8, right : u8 ) -> u8
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
pub fn ascii_string_to_u32(s : &str) -> u32
{
    let datain = s.as_bytes();
    u8s_string_to_u32(datain)
}
pub fn u8s_string_to_u32(datain : &[u8]) -> u32
{
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


pub fn take_adress_length( xin : &str ) -> Option< (u32, u32) >
{
    let args : Vec <&str>= xin.split(",").collect();
    if args.len()!=2
    {
        bmplog!("take_adress_length : wrong param");
        return None;
    }
    let address = crate::parsing_util::ascii_string_to_u32(args[0]);
    let len = crate::parsing_util::ascii_string_to_u32(args[1]);
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