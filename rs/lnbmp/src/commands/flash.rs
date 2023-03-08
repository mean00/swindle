
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::util::hex_to_u8s;

use crate::encoder::encoder;
use super::{CommandTree,exec_one};

use crate::bmp::{bmp_attach,bmp_flash_erase};
use crate::commands::CallbackType;

const v_command_tree: [CommandTree;3] = 
[
    CommandTree{ command: "vFlashErase",args: 0, require_connected: true,  cb: CallbackType::text( _vFlashErase) },  // flash erase
    CommandTree{ command: "vFlashWrite",args: 0, require_connected: true,  cb: CallbackType::raw(  _vFlashWrite) },  // flash write
    CommandTree{ command: "vFlashDone ",args: 0, require_connected: true,  cb: CallbackType::text( _vFlashDone)  },  // flash erase
];


//
//
//

pub fn _flashv(command : &str, args : &[u8]) -> bool
{
    return exec_one(&v_command_tree,command,args);
}

//
//vFlashErase:08000000,00005000
fn _vFlashErase(command : &str, args : &Vec<&str>) -> bool
{    
    let xin = &args[0];
    match crate::util::take_adress_length(&xin[1..])
    {
        None =>   encoder::reply_e01(),
        Some( (adr,len) ) => 
            {    
                if bmp_flash_erase(adr,len)
                {
                    encoder::reply_ok();
                }else
                {
                    encoder::reply_e01();
                }
            },
    };
    return true;
}
//
//vFlashWrite:08000000,data
fn _vFlashWrite(command : &str, args : &[u8]) -> bool
{    
    
    return false;
}
//vFlashDone
fn _vFlashDone(command : &str, args : &Vec<&str>) -> bool
{    
    
    return false;
}


// EOF