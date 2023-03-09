
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
        Some( (adr,len) ) => 
            {
                
                let mut tmp  = [0];
                let mut char_buffer : [u8;2]  =[ 0, 0];
                let mut e  = encoder::new();
                e.begin();
                for i in 0..len
                {
                    crate::bmp::bmp_read_mem(adr+i,&mut tmp);
                    crate::util::u8_to_ascii_to_buffer(tmp[0],&mut char_buffer);
                    e.add_u8(&char_buffer); // handle error ?
                }
                e.end();
                
            }
    }
    return true;
  
}

// EOF