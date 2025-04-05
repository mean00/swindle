use crate::bmp;
use crate::commands::mon::MAX_SPACE;
use crate::commands::mon::spacebar;
use crate::commands::{CallbackType, CommandTree, HelpTree, exec_one};
use crate::encoder::encoder;
use crate::rn_bmp_cmd_c::rttField_ADDRESS;
use crate::rn_bmp_cmd_c::rttField_ENABLED;
use crate::rtt;
use numtoa::NumToA;

crate::setup_log!(false);
crate::gdb_print_init!();
use crate::gdb_print;

use crate::rtt::{get_rtt_info, set_rtt_info};
/*
 *
 */
const rtt_command_tree: [CommandTree; 6] = [
    CommandTree {
        command: "help",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_help),
        start_separator: " ",
        next_separator: " ",
    },
    CommandTree {
        command: "enable",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_enable),
        start_separator: " ",
        next_separator: " ",
    },
    CommandTree {
        command: "disable",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_disable),
        start_separator: " ",
        next_separator: " ",
    },
    CommandTree {
        command: "status",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_status),
        start_separator: " ",
        next_separator: " ",
    },
    CommandTree {
        command: "cblock",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_cblock),
        start_separator: " ",
        next_separator: " ",
    },
    CommandTree {
        command: "ram",
        min_args: 2,
        require_connected: true,
        cb: CallbackType::text(_scan),
        start_separator: " ",
        next_separator: " ",
    },
];
/*
 *
 */
const rtt_help_tree: [HelpTree; 6] = [
    HelpTree {
        command: "help",
        help: "Display help.",
    },
    HelpTree {
        command: "enable",
        help: "enable rtt.",
    },
    HelpTree {
        command: "disable",
        help: "disable rtt .",
    },
    HelpTree {
        command: "status",
        help: "print status.",
    },
    HelpTree {
        command: "cblock",
        help: "printf control block info .",
    },
    HelpTree {
        command: "ram startadr endadr",
        help: "scan ram to find the RTT block.",
    },
];
/*
 * it should start mon rtt xxxx
 */
#[unsafe(no_mangle)]
pub fn _rtt(_command: &str, args: &[&str]) -> bool {
    let as_string = args[0..].join(" ");
    let as_str: &str = &as_string;
    exec_one(&rtt_command_tree, as_str, &[])
}

/*
 *
 */
fn _help(_command: &str, _args: &[&str]) -> bool {
    let mut mxsize: usize = 0;
    for i in rtt_help_tree {
        if i.command.len() > mxsize {
            mxsize = i.command.len();
        }
    }
    if mxsize > MAX_SPACE {
        panic!("padding too big");
    }
    for i in rtt_help_tree {
        let cmd = i.command;
        let len = cmd.len();
        let pad = core::str::from_utf8(&spacebar[..(mxsize - len)]).unwrap();
        gdb_print!("mon rtt {}{} : {}\n", &cmd, &pad, &(i.help));
    }
    encoder::reply_ok();
    true
}
/*
 *
 */
fn _enable(_command: &str, _args: &[&str]) -> bool {
    let mut info = get_rtt_info();
    info.enabled = 1;
    set_rtt_info(rttField_ENABLED, &info);
    encoder::reply_ok();
    true
}
/*
 *
 */
fn _disable(_command: &str, _args: &[&str]) -> bool {
    let mut info = get_rtt_info();
    info.enabled = 0;
    set_rtt_info(rttField_ENABLED, &info);
    encoder::reply_ok();
    true
}
/*
 *
 */
fn TrueFalse(onoff: u32) -> &'static str {
    if onoff == 0 { "false" } else { "true" }
}
/*
 *
 */
fn _status(_command: &str, _args: &[&str]) -> bool {
    let mut info = get_rtt_info();
    gdb_print!("Rtt : \n");
    gdb_print!("\tEnabled   :{}\n", TrueFalse(info.enabled));
    gdb_print!("\tFound     :{}\n", TrueFalse(info.found));
    gdb_print!("\tMinAddress:{:x}\n", info.min_address);
    gdb_print!("\tMaxAddress:{:x}\n", info.max_address);
    encoder::reply_ok();
    true
}
/*
 *
 */
fn _cblock(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_ok();
    true
}
/*
 * scan ram to find the RTT block...
 * Ram BEGIN_ADDR END_ADDR
 */
fn _scan(_command: &str, args: &[&str]) -> bool {
    let mut start: u32 = crate::parsing_util::ascii_hex_or_dec_to_u32(args[0]);
    let mut end: u32 = crate::parsing_util::ascii_hex_or_dec_to_u32(args[1]);
    if end <= start {
        gdb_print!("Invalid address\n");
        encoder::reply_e01();
        return true;
    }
    let mut info = get_rtt_info();
    info.min_address = start;
    info.max_address = end;
    set_rtt_info(rttField_ADDRESS, &info);
    encoder::reply_ok();
    true
}
// EOF
