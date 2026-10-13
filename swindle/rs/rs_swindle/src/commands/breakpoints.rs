//! GDB `Z`/`z` breakpoint and watchpoint commands.
//!
//! Handles the GDB remote protocol breakpoint commands:
//!
//! - `Z0,addr,kind` — set software breakpoint
//! - `Z1,addr,kind` — set hardware breakpoint
//! - `Z2,addr,kind` — set write watchpoint
//! - `Z3,addr,kind` — set read watchpoint
//! - `Z4,addr,kind` — set access watchpoint
//! - `z0,addr,kind` — remove software breakpoint
//! - `z1,addr,kind` — remove hardware breakpoint
//! - etc.
//!
//! Falls back to mass-write (flash patching) if the target has no hardware
//! breakpoint support.


use crate::bmp;
use crate::encoder::encoder;

use crate::parsing_util::ascii_string_hex_to_u32;

setup_log!(false);
//use crate::bmplog;
use crate::sw_breakpoints::{add_mw_breakpoint, remove_mw_breakpoint};
use crate::sw_breakpoints::{add_sw_breakpoint, remove_sw_breakpoint};

/// Breakpoint/watchpoint type, matching the GDB remote protocol values.
///
/// Same values as BMP internal:
/// - 0 = TARGET_BREAK_SOFT
/// - 1 = TARGET_BREAK_HARD
/// - 2 = TARGET_WATCH_WRITE
/// - 3 = TARGET_WATCH_READ
/// - 4 = TARGET_WATCH_ACCESS
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
/// Common handler for both set (`Z`) and remove (`z`) breakpoint commands.
///
/// Args: `[type],address,kind`
fn common_z(set: bool, args: &[&str]) -> bool {
    // zZ addr kind
    let breakpoint_watchpoint = Breakpoints::from_int(ascii_string_hex_to_u32(args[0]));
    let address: u32 = ascii_string_hex_to_u32(args[1]);
    // ignore "kind"
    let len: u32 = 4;

    if !bmp::bmp_has_hw_breakpoint() {
        if !bmp::bmp_has_mw_helpers() {
            encoder::reply_e01();
            return true;
        }

        // mw breakpoint
        let ok = if set {
            add_mw_breakpoint(address)
        } else {
            remove_mw_breakpoint(address)
        };
        encoder::reply_bool(ok);
        return true;
    }

    if breakpoint_watchpoint == Breakpoints::Execute_SW {
        let ok = if set {
            add_sw_breakpoint(address, len)
        } else {
            remove_sw_breakpoint(address, len)
        };
        encoder::reply_bool(ok);
        return true;
    }

    let bmp_type = Breakpoints::to_bmp(&breakpoint_watchpoint);
    let ok = if set {
        bmplog!("adding breakpoint at 0x{:x}\n", address);
        bmp::bmp_add_breakpoint(bmp_type, address, len)
    } else {
        bmplog!("removing breakpoint at 0x{:x}\n", address);
        bmp::bmp_remove_breakpoint(bmp_type, address, len)
    };
    encoder::reply_bool(ok);
    true
}
/*
 *
 */
/// Handle `z` (remove breakpoint/watchpoint).
pub fn _z(_command: &str, args: &[&str]) -> bool {
    bmplog!("remove bkp\n");
    common_z(false, args)
}
/*
 *
 */
/// Handle `Z` (set breakpoint/watchpoint).
pub fn _Z(_command: &str, args: &[&str]) -> bool {
    bmplog!("insert bkp\n");
    common_z(true, args)
}
// EOF
