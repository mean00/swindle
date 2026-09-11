//! GDB `v` packet handlers (attach, run, must-reply).
//!
//! Implements the GDB remote protocol `v` packet family:
//!
//! | Packet | Description |
//! |--------|-------------|
//! | `vAttach;target` | Attach to a target on the SWD/RISC-V bus |
//! | `vRun` | Start the program: reset the target, report the entry stop |
//! | `vMustReply` | Empty reply (LLDB compatibility) |
//!
//! On attach, this module:
//! 1. Connects to the target
//! 2. Registers target-specific monitor commands
//! 3. Resets symbol state for FreeRTOS/RTT lookup
//! 4. Clears any previous software breakpoints
//!
//! ## References
//!
//! - <https://sourceware.org/gdb/onlinedocs/gdb/Packets.html>
//! - <https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html>


use super::{CommandTree, exec_one};
use crate::bmp;
use crate::bmp::bmp_attach;
use crate::commands::CallbackType;
use crate::commands::mon;
use crate::commands::symbols;
use crate::encoder::encoder;
use crate::freertos::os_detach;
use crate::sw_breakpoints;

setup_log!(true);
use crate::parsing_util::ascii_string_decimal_to_u32;

const v_command_tree: [CommandTree; 3] = [
    CommandTree {
        command: "vMustReply",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_vMustReply),
        start_separator: 0,
        next_separator: 0,
    }, // test
    CommandTree {
        command: "vAttach",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_vAttach),
        start_separator: b';',
        next_separator: 0,
    }, // test
    CommandTree {
        command: "vRun",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(vRun),
        start_separator: 0,
        next_separator: 0,
    }, // test
];
//
//
/// Handle `vRun` — GDB's "run": reset the target, report the entry-point stop.
///
/// Upstream blackmagic does both halves of this in `exec_v_run()`
/// (`blackmagic/src/gdb_main.c`):
///
/// ```c
/// /* Run target program. For us (embedded) this means reset. */
/// target_reset(cur_target);
/// gdb_put_packet_str("T05");
/// ```
///
/// GDB relies on both. The reset puts the program counter on the entry point,
/// and the stop reply tells GDB that the target is *stopped*, so it goes on to
/// insert its breakpoints and only then resumes with `vCont;c`. Replying a bare
/// `OK` (as this port used to) left the target un-reset and never reported a
/// stop, so `load` + `b main` + `run` never reached `main`, while `continue`
/// worked because it enters through `vCont;c` and needs no reset.
///
/// This matters most for the CH32V003, the only target with
/// `no_hw_breakpoint` set (`blackmagic_addon/target/CH32V0xx/ch32v0x.c`): its
/// breakpoints are flash pages patched through the mass-write helpers, and
/// that patch has to be in place while the hart is parked on the entry point.
fn vRun(_command: &str, _args: &[&str]) -> bool {
    crate::mem_cache::invalidate();
    bmp::bmp_reset_target();
    crate::commands::run::report_reset_stop();
    true
}
//
//
//

/// Dispatch `v*` packets to the appropriate handler.
pub fn _v(command: &[u8]) -> bool {
    exec_one(&v_command_tree, command)
}

//
//
//
/// Handle `vMustReply` — empty reply (LLDB compatibility).
fn _vMustReply(_command: &str, _args: &[&str]) -> bool {
    encoder::simple_send("");
    true
}
//
//
//
/// Handle `vAttach;target` — attach to a target on the debug bus.
///
/// On success, registers target-specific commands and sends a stop
/// reply with thread 1 (required for GDB 11/12 compatibility).
fn _vAttach(_command: &str, args: &[&str]) -> bool {
    // The string is normally vAttach;XXX
    if args.len() != 1 {
        return false;
    }
    let mut target = ascii_string_decimal_to_u32(args[0]);
    if target == 0 {
        target = 1;
    }
    bmplog!("Attaching to {}  \n", target);
    if bmp_attach(target) {
        /*
         * We don't actually support threads, but GDB 11 and 12 can't work without
         * us saying we attached to thread 1.. see the following for the low-down of this:
         * https://sourceware.org/bugzilla/show_bug.cgi?id=28405
         * https://sourceware.org/bugzilla/show_bug.cgi?id=28874
         * https://sourceware.org/pipermail/gdb-patches/2021-December/184171.html
         * https://sourceware.org/pipermail/gdb-patches/2022-April/188058.html
         * https://sourceware.org/pipermail/gdb-patches/2022-July/190869.html
         */
        mon::add_target_commands(bmp::bmp_get_target_name());
        encoder::simple_send("T05thread:1;");
        return true;
    }
    bmplog!("Attach failed\n");
    symbols::reset_symbols();
    os_detach();
    sw_breakpoints::clear_sw_breakpoint();
    mon::clear_custom_target_command();
    encoder::reply_e01();
    true
}

// EOF
