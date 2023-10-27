
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use crate::encoder::encoder;
use super::{CommandTree,exec_one};

use crate::bmp::{bmp_attach,bmp_flash_erase,bmp_flash_write, bmp_flash_complete};
use crate::commands::CallbackType;

use crate::parsing_util::ascii_string_to_u32;

use crate::bmp;

use alloc::vec;
use alloc::vec::Vec;

crate::setup_log!(false);
use crate::{bmplog,bmpwarning};
/*
Same value as bmp internal
	TARGET_BREAK_SOFT 0 ,
	TARGET_BREAK_HARD 1,
	TARGET_WATCH_WRITE 2,
	TARGET_WATCH_READ 3,
	TARGET_WATCH_ACCESS 4,

 */
enum Breakpoints
{
    Execute,
    Read,
    Write,
    Access,
}
impl Breakpoints
{
    pub fn from_int( val : u32 )  -> Self
    {
        match val
        {
            0 | 1   => Breakpoints::Execute,
            2       => Breakpoints::Write,
            3       => Breakpoints::Read,
            4       => Breakpoints::Access,
            _ => panic!("Invalid watchpoint"),
        }
    }
    pub fn to_int( b: &Self) -> u32
    {
        match b
        {
            Breakpoints::Execute => 1,
            Breakpoints::Read    => 3,
            Breakpoints::Write   => 2,
            Breakpoints::Access  => 4,
        }
    }
}

fn common_z(command : &str) -> bool
{
    let args : Vec <&str>= command.split(',').collect();
    if args.len()!=3
    {
        encoder::reply_e01();
        return true;
    }
    // zZ addr kind
    let prefix = args[0];
    let breakpoint_watchpoint = Breakpoints::from_int( ascii_string_to_u32( &prefix[1..2]));
    let address : u32 = ascii_string_to_u32(args[1]);    
    let len : u32 = 4;

    if args[0].starts_with('z') // remove
    {
        encoder::reply_bool( crate::bmp::bmp_remove_breakpoint(Breakpoints::to_int(&breakpoint_watchpoint), address,len) );
        return true;
    }
    if args[0].starts_with('Z') // add
    {
        encoder::reply_bool( crate::bmp::bmp_add_breakpoint(Breakpoints::to_int(&breakpoint_watchpoint), address,len) );
        return true;            
    }
    encoder::reply_e01();
    true
}

pub fn _z(command : &str, _args : &[&str]) -> bool
{    
  common_z(command)
}

pub fn _Z(command : &str, _args :&[&str]) -> bool
{    
    common_z(command)
}

// EOF