// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use super::{CommandTree, exec_one};
use crate::bmp;
use crate::bmp::bmp_attach;
use crate::commands::CallbackType;
use crate::commands::symbols;
use crate::encoder::encoder;
use crate::freertos::os_detach;

crate::setup_log!(false);
use crate::parsing_util::ascii_string_decimal_to_u32;

const v_command_tree: [CommandTree; 3] = [
    CommandTree {
        command: "vMustReply",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_vMustReply),
        start_separator: "",
        next_separator: "",
    }, // test
    CommandTree {
        command: "vAttach",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_vAttach),
        start_separator: ";",
        next_separator: "",
    }, // test
    CommandTree {
        command: "vRun",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(vRun),
        start_separator: "",
        next_separator: "",
    }, // test
];
//
//
fn vRun(_command: &str, _args: &[&str]) -> bool {
    // _vCont("vCont",args)
    encoder::reply_ok();
    true
}
//
//
//

pub fn _v(command: &str, args: &[u8]) -> bool {
    exec_one(&v_command_tree, command, args)
}

//
//
//
fn _vMustReply(_command: &str, _args: &[&str]) -> bool {
    encoder::simple_send("");
    true
}
//
//
//
fn _vAttach(_command: &str, args: &[&str]) -> bool {
    // The string is normally vAttach;XXX
    if args.len() != 1 {
        return false;
    }
    let mut target = ascii_string_decimal_to_u32(args[0]);
    if target == 0 {
        target = 1;
    }
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
        crate::commands::mon::add_target_commands(bmp::bmp_get_target_name());
        encoder::simple_send("T05thread:1;");
        return true;
    }
    symbols::reset_symbols();
    os_detach();
    crate::sw_breakpoints::clear_sw_breakpoint();
    crate::commands::mon::clear_custom_target_command();
    encoder::reply_e01();
    true
}

// EOF
