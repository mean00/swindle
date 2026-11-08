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
//! Falls back to patching the code itself (flash page rewrite or RAM write) when the
//! target has no hardware breakpoint support, or when the target declines one.


use crate::bmp;
use crate::encoder::encoder;
use crate::target::BreakwatchType;

use crate::parsing_util::ascii_string_hex_to_u32;

setup_log!(false);
//use crate::bmplog;
use crate::sw_breakpoints::{add_ram_breakpoint, add_soft_breakpoint};
use crate::sw_breakpoints::{remove_ram_breakpoint, remove_soft_breakpoint};

/// Patch the code at `address` itself: through the flash controller for flash,
/// with a plain memory write for RAM.
///
/// The choice is made from the address, not from the target's trigger support: an
/// `ebreak`/`BKPT` written into flash is silently dropped by the flash controller, so a
/// target that *does* have hardware breakpoints still needs the page rewrite for its
/// flash regions.
fn common_z_patch(set: bool, address: u32) -> bool {
    // A target without page helpers cannot rewrite a flash page; the plain write is all
    // that is left (and what this handler did before the address check existed).
    if bmp::bmp_is_flash_addr(address) && bmp::bmp_has_mw_helpers() {
        return if set {
            add_soft_breakpoint(address)
        } else {
            remove_soft_breakpoint(address)
        };
    }
    if set {
        add_ram_breakpoint(address, 4)
    } else {
        remove_ram_breakpoint(address, 4)
    }
}

/// Common handler for both set (`Z`) and remove (`z`) breakpoint commands.
///
/// Args: `[type],address,kind`
fn common_z(set: bool, args: &[&str]) -> bool {
    // zZ addr kind
    let Some(kind) = BreakwatchType::from_int(ascii_string_hex_to_u32(args[0])) else {
        encoder::reply_e01();
        return true;
    };
    let address: u32 = ascii_string_hex_to_u32(args[1]);
    // ignore "kind"
    let len: usize = 4;

    // Software breakpoint: always a patch of the code, never a hardware trigger.
    if kind == BreakwatchType::BreakSoft {
        encoder::reply_bool(common_z_patch(set, address));
        return true;
    }

    // Hardware breakpoint / watchpoint: the target's trigger unit gets first refusal,
    // and a target that has none (or declines: it answers "unsupported")
    // falls back to patching the code, which is what GDB asked for in the end.
    let result = if set {
        bmplog!("adding breakpoint at 0x{:x}\n", address);
        crate::target::breakwatch::set(kind, address, len)
    } else {
        bmplog!("removing breakpoint at 0x{:x}\n", address);
        crate::target::breakwatch::clear(kind, address, len)
    };
    if result.is_ok() {
        encoder::reply_ok();
        return true;
    }

    // An execute breakpoint can still be a code patch. A watchpoint cannot: there is
    // nothing to patch, so report the failure instead of corrupting memory.
    if kind == BreakwatchType::BreakHard {
        encoder::reply_bool(common_z_patch(set, address));
        return true;
    }
    encoder::reply_e01();
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
