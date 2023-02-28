
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::util::hex_to_u8s;

use crate::encoder::encoder;
use super::{CommandTree,exec_one};

use crate::bmp::bmp_attach;

const v_command_tree: [CommandTree;2] = 
[
    CommandTree{ command: "vMustReply", args: 0, cb: _vMustReply },  // test
    CommandTree{ command: "vAttach",    args: 0, cb: _vAttach },  // test
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
    encoder::simple_send("E01");
    true
}

// EOF