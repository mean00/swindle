
use alloc::vec;
use alloc::vec::Vec;

use crate::encoder::encoder;
use super::{CommandTree,exec_one};
use crate::packet_symbols::INPUT_BUFFER_SIZE;

use super::mon::{_swdp_scan};
use crate::bmp::bmp_attached;
use crate::bmp::bmp_get_mapping;
use crate::bmp::MemoryBlock;
use crate::bmp::mapping::{ FLASH,RAM};

use crate::parsing_util::{ ascii_string_to_u32_le, ascii_string_to_u32 };

use crate::bmp;

use numtoa::NumToA;

crate::setup_log!(false);

// write register
// Pf=40000008
//

fn pp_prefix( command : &str) -> Option<(u32,u32)>
{       
    let args : Vec <&str>= command.split('=').collect();
    if args.len()!=2
    {
        bmplog("Pxxx wrong args");
        return None;
    }
    let reg = ascii_string_to_u32(args[0]);
    let value = ascii_string_to_u32_le(args[1]);
    return Some( (reg,value));
}
// Write reg
pub fn _P(command : &str, _args : &Vec<&str>) -> bool
{
    let reg : u32;
    let val : u32;
    match pp_prefix(&command[1..])
    {
        None =>  { encoder::simple_send("E01");return true },
        Some( (x,y) )=>  {reg=x;val=y;},
    }   
    encoder::reply_bool(   bmp::bmp_write_register(reg,val) );
    return true;
}
// read 1 register
pub fn _p(command : &str, _args : &Vec<&str>) -> bool
{
    let reg : u32 = crate::parsing_util::ascii_string_to_u32( &command[1..]);   
    match bmp::bmp_read_register(reg)
    {
        Some(x) => encoder::simple_send_u32_le(x),
        _            => encoder::reply_e01()
    }
    true
}

// Read all registers
pub fn _g(_command : &str, _args : &Vec<&str>) -> bool
{       

    let regs = crate::bmp::bmp_read_registers();
    let mut e = encoder::new();
        
    let n: usize = regs.len();
    if n==0
    {
        encoder::simple_send("0000");
        true;
    }
    e.begin();
    for i in 0..n
    {               
        e.add_u32_le( regs[i]);
    }
    // now read CSRs
    // this is hackish, we should ask bmp for for the csr
    // TODO FIXME
    let csr = vec![   0x300,   0x301,  0x304,    0x305,       0x340,       0x341,        0x342,        0x343,               0x344];
    for r in csr
    {
        match crate::bmp::bmp_read_register(r+128)
        {
            Some(x) =>  e.add_u32_le(x),
            None =>  e.add_u32_le(0),
        };
    }
    e.end();
    true
}

