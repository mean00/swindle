// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use crate::encoder::encoder;

use crate::bmp::{bmp_attach, bmp_flash_complete, bmp_flash_erase, bmp_flash_write};
use crate::commands::CallbackType;

use crate::parsing_util::ascii_string_hex_to_u32;

use crate::bmp;

use alloc::vec;
use alloc::vec::Vec;

crate::setup_log!(false);
use crate::sw_breakpoints::{add_sw_breakpoint, remove_sw_breakpoint};
use crate::{bmplog, bmpwarning};

/*
Same value as bmp internal
    TARGET_BREAK_SOFT 0 ,
    TARGET_BREAK_HARD 1,
    TARGET_WATCH_WRITE 2,
    TARGET_WATCH_READ 3,
    TARGET_WATCH_ACCESS 4,

 */
#[derive(PartialEq, Clone, Copy)]
enum Breakpoints {
    Execute_SW,
    Execute_HW,
    Read,
    Write,
    Access,
}
impl Breakpoints {
    pub fn from_int(val: u32) -> Self {
        match val {
            0 => Breakpoints::Execute_SW,
            1 => Breakpoints::Execute_HW,
            2 => Breakpoints::Write,
            3 => Breakpoints::Read,
            4 => Breakpoints::Access,
            _ => panic!("Invalid watchpoint"),
        }
    }
    pub fn to_bmp(b: &Self) -> u32 {
        match b {
            Breakpoints::Execute_SW => 0,
            Breakpoints::Execute_HW => 1,
            Breakpoints::Read => 3,
            Breakpoints::Write => 2,
            Breakpoints::Access => 4,
        }
    }
}
// : Z1,776,2
// if set is true, set breakpoint
// if false remove
// [HW/SW/WATCH],address,kind
fn common_z(set: bool, args: &[&str]) -> bool {
    // zZ addr kind
    let breakpoint_watchpoint = Breakpoints::from_int(ascii_string_hex_to_u32(args[0]));
    let address: u32 = ascii_string_hex_to_u32(args[1]);
    // ignore "kind"
    let len: u32 = 4;

    if set
    // add
    {
        //--
        //--
        if breakpoint_watchpoint == Breakpoints::Execute_SW {
            encoder::reply_bool(add_sw_breakpoint(address, len));
            return true;
        }
        bmplog!("adding breakpoint at 0x{:x}\n", address);
        encoder::reply_bool(crate::bmp::bmp_add_breakpoint(
            Breakpoints::to_bmp(&breakpoint_watchpoint),
            address,
            len,
        ));
        //--
        return true;
    }
    // remove
    if breakpoint_watchpoint == Breakpoints::Execute_SW {
        encoder::reply_bool(remove_sw_breakpoint(address, len));
        return true;
    }
    bmplog!("removing breakpoint at 0x{:x}\n", address);
    encoder::reply_bool(crate::bmp::bmp_remove_breakpoint(
        Breakpoints::to_bmp(&breakpoint_watchpoint),
        address,
        len,
    ));
    true
}
/*
 *
 */
pub fn _z(_command: &str, args: &[&str]) -> bool {
    bmplog!("remove bkp\n");
    common_z(false, args)
}
/*
 *
 */
pub fn _Z(_command: &str, args: &[&str]) -> bool {
    bmplog!("insert bkp\n");
    common_z(true, args)
}
// EOF
