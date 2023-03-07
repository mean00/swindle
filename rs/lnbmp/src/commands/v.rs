
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::util::hex_to_u8s;

use crate::encoder::encoder;
use super::{CommandTree,exec_one};

use crate::bmp::{bmp_attach,bmp_flash_erase};

const v_command_tree: [CommandTree;3] = 
[
    CommandTree{ command: "vMustReply", args: 0, require_connected: false, cb: _vMustReply },  // test
    CommandTree{ command: "vAttach",    args: 0, require_connected: false, cb: _vAttach },  // test
    CommandTree{ command: "vFlashErase",args: 0, require_connected: true, cb: _vFlashErase },  // flash erase
];


//
//
//

pub fn _v(tokns : &Vec<&str>) -> bool
{
    return exec_one(&v_command_tree,tokns);
}

//
//
//
fn _vMustReply(_tokns : &Vec<&str>) -> bool
{
    encoder::simple_send("");    
    true
}
//
//
//
fn _vAttach(_tokns : &Vec<&str>) -> bool
{
    if bmp_attach(1)
    {
            /*
			 * We don't actually support threads, but GDB 11 and 12 can't work without
			 * us saying we attached to thread 1.. see the following for the low-down of this:
			 * https://sourceware.org/bugzilla/show_bug.cgi?id=28405
			 * https://sourceware.org/bugzilla/show_bug.cgi?id=28874
			 * https://sourceware.org/pipermail/gdb-patches/2021-December/184171.html
			 * https://sourceware.org/pipermail/gdb-patches/2022-April/188058.html
			 * https://sourceware.org/pipermail/gdb-patches/2022-July/190869.html
			 */
             encoder::simple_send("T05thread:1;");
             return true;
    }
    encoder::reply_e01();
    true
}
//vFlashErase:08000000,00005000
fn _vFlashErase(_tokns : &Vec<&str>) -> bool
{    
    let xin = &_tokns[1];
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


// EOF