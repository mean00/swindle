// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use super::{exec_one, CommandTree};
use crate::bmp::{bmp_attach, bmp_cpuid};
use crate::commands::CallbackType;
use crate::commands::_vCont;
use crate::encoder::encoder;
use crate::freertos::os_attach;
use crate::freertos::os_detach;

crate::setup_log!(false);
use crate::{bmplog, bmpwarning};

const v_command_tree: [CommandTree; 3] = [
    CommandTree {
        command: "vMustReply",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_vMustReply),
    }, // test
    CommandTree {
        command: "vAttach",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_vAttach),
    }, // test
    CommandTree {
        command: "vRun",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(vRun),
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
fn _vAttach(_command: &str, _args: &[&str]) -> bool {
    if bmp_attach(1) {
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
    os_detach();
    encoder::reply_e01();
    true
}

// EOF
