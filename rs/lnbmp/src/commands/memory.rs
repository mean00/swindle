
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::util::hex_to_u8s;

use crate::encoder::encoder;
use super::{CommandTree,exec_one};

use crate::bmp::{bmp_attach,bmp_flash_erase};

// memory read m80070f6,4
pub fn _m(command : &str, _args : &Vec<&str>) -> bool
{
    if !crate::bmp::bmp_attached()
    {
        encoder::reply_e01(); 
        return true;
    }
    
    match crate::util::take_adress_length(&command[1..])
    {
        None => encoder::reply_e01(),
        Some( (_adr,_len) ) => 
            {
                encoder::reply_e01()
            }
    }
    return true;
  
}

// EOF