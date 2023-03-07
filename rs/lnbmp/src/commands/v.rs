
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::util::hex_to_u8s;

use crate::encoder::encoder;
use super::{CommandTree,exec_one};

use crate::bmp::bmp_attach;

const v_command_tree: [CommandTree;3] = 
[
    CommandTree{ command: "vMustReply", args: 0, cb: _vMustReply },  // test
    CommandTree{ command: "vAttach",    args: 0, cb: _vAttach },  // test
    CommandTree{ command: "vFlashErase",args: 0, cb: _vFlashErase },  // flash erase
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
    if !crate::bmp::bmp_attached()
    {
        encoder::reply_e01(); 
        return true;
    }

    let args : Vec <&str>= _tokns[1].split(",").collect();
    if args.len()!=2
    {
        glog("vflasherase : wrong param");
        encoder::reply_e01(); 
        return true;
    }
    let address = crate::util::ascii_to_u32(args[0]);
    let len = crate::util::ascii_to_u32(args[1]);
    return false;
}


// EOF