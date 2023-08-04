
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

use crate::parsing_util::ascii_string_to_u32_le;

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
    let reg = ascii_string_to_u32_le(args[0]);
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


// Read registers
pub fn _g(_command : &str, _args : &Vec<&str>) -> bool
{       

    let regs = crate::bmp::bmp_read_registers();
    let mut e = encoder::new();
    
    let mut buffer : [u8;8]=[0;8];
    let n: usize = regs.len();
    if n==0
    {
        encoder::simple_send("0000");
        true;
    }
    e.begin();
    for i in 0..n
    {       
        let mut reg = regs[i];
        // LE first
        for j in 0..4
        {
            crate::parsing_util::u8_to_ascii_to_buffer((reg &0xff) as u8 ,&mut buffer[2*j..]); 
            reg = reg >> 8;
        }
        e.add_u8(&buffer);
    }
    e.end();
    true
}

