
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::util::glogx;
use crate::util::hex_to_u8s;

use crate::encoder::encoder;
use super::{CommandTree,exec_one};

use crate::bmp::{bmp_attach,bmp_flash_erase,bmp_flash_write, bmp_flash_complete};
use crate::commands::CallbackType;


fn common_z(command : &str) -> bool
{
    let args : Vec <&str>= command.split(",").collect();
    if args.len()!=3
    {
        encoder::reply_e01();
        return true;
    }
    // zZ addr kind
    let address : u32 = crate::util::ascii_to_u32(args[1]);
    // we dont care, we always use 1 i.e. hw breakpoint
    let kind : u32 = crate::util::ascii_to_u32(args[2]);
    let len : u32 = 4;
    if args[0].starts_with("z") // remove
    {
        encoder::reply_bool( crate::bmp::bmp_remove_breakpoint(1, address,len) );        
    }
    else
    if args[0].starts_with("Z") // add
    {
        encoder::reply_bool( crate::bmp::bmp_add_breakpoint(1, address,len) );
    }else
    {
        encoder::reply_e01();
    }
    true
}

pub fn _z(command : &str, _args : &Vec<&str>) -> bool
{    
  common_z(command)
}

pub fn _Z(command : &str, _args : &Vec<&str>) -> bool
{    
    common_z(command)
}

// EOF