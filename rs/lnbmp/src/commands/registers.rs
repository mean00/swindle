
use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::util::hex_to_u8s;

use crate::encoder::encoder;
use super::{CommandTree,exec_one};
use crate::packet_symbols::INPUT_BUFFER_SIZE;

use super::mon::{_swdp_scan};
use crate::bmp::bmp_attached;
use crate::bmp::bmp_get_mapping;
use crate::bmp::MemoryBlock;
use crate::bmp::mapping::{ FLASH,RAM};

use crate::util::ascii_to_u32;

use crate::bmp;

use numtoa::NumToA;
// write register
// Pf=40000008
//

fn pp_prefix( command : &str) -> Option<(u32,u32)>
{       
    let args : Vec <&str>= command.split('=').collect();
    if args.len()!=2
    {
        glog("Pxxx wrong args");
        return None;
    }
    let reg = ascii_to_u32(args[0]);
    let value = ascii_to_u32(args[1]);
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

