
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
pub fn _m(tokns : &Vec<&str>) -> bool
{
    if !crate::bmp::bmp_attached()
    {
        encoder::reply_e01(); 
        return true;
    }
    let xin = &tokns[0];
    match crate::util::take_adress_length(&xin[1..])
    {
        None => encoder::reply_e01(),
        Some( (adr,len) ) => 
            {
                encoder::reply_e01()
            }
    }
    return true;
  
}

// EOF