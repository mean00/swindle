
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::util::glogx;
use crate::parsing_util::hex_to_u8s;
use crate::parsing_util::ascii_to_u32;
use crate::encoder::encoder;
use super::{CommandTree,exec_one};

use crate::bmp::{bmp_attach,bmp_flash_erase,bmp_flash_write, bmp_flash_complete};
use crate::commands::CallbackType;
use numtoa::NumToA;

use crate::bmp;

pub enum HaltState
{
    Running     ,
    Error       ,
    Request     ,
    Stepping    ,
    Breakpoint  ,
    Watchpoint(u32)  ,
    Fault       ,
}

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
        return match val
        {
            0 | 1   => Breakpoints::Execute,
            2       => Breakpoints::Write,
            3       => Breakpoints::Read,
            4       => Breakpoints::Access,
            _ => panic!("Invalid watchpoint"),
        };
    }
    pub fn to_int( b: &Self) -> u32
    {
        return match b
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
    let args : Vec <&str>= command.split(",").collect();
    if args.len()!=3
    {
        encoder::reply_e01();
        return true;
    }
    // zZ addr kind
    let prefix = args[0];
    let breakpoint_watchpoint = Breakpoints::from_int( ascii_to_u32( &prefix[1..2]));
    let address : u32 = ascii_to_u32(args[1]);    
    let len : u32 = 4;

    if args[0].starts_with("z") // remove
    {
        encoder::reply_bool( crate::bmp::bmp_remove_breakpoint(Breakpoints::to_int(&breakpoint_watchpoint), address,len) );        
    }
    else
    if args[0].starts_with("Z") // add
    {
        encoder::reply_bool( crate::bmp::bmp_add_breakpoint(Breakpoints::to_int(&breakpoint_watchpoint), address,len) );
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

//
fn reply_2( prefix : &str, num : u32)
{
    let mut buffer: [u8;20] = [0; 20]; 
    let mut e = crate::encoder::encoder::new();
    e.begin();
    e.add(prefix);
    e.add(num.numtoa_str(16,&mut buffer));  
    e.end();
}
//
fn reply_4( prefix : &str, num : u32, prefix2 : &str, num2: u32)
{
    let mut buffer: [u8;20] = [0; 20]; 
    let mut e = crate::encoder::encoder::new();
    e.begin();
    e.add(prefix);
    e.add(num.numtoa_str(16,&mut buffer));  
    e.add(prefix2);
    e.add(num2.numtoa_str(16,&mut buffer));  
    e.end();
}

#[no_mangle]
extern "C" fn rngdbstub_poll()
{
    // this is called regularily
    if bmp::bmp_attached()
    {
        match bmp::bmp_poll()
        {
            HaltState::Running      => (),                  // nothing to do !
            HaltState::Error        => reply_2("X", 29),    // SIGLOST
            HaltState::Request      => reply_2("T", 2),     // SIGINT
            HaltState::Watchpoint(wp)  => reply_4("T", 5, "watch:",wp as u32 ), // SIGTRAP
            HaltState::Fault        => reply_2("T", 11),  // SIGSEGV
            HaltState::Breakpoint   => reply_2("T", 5), // SIGTRAP
            _ => panic!("unsupported halt state"),
        }
    }
}
// EOF