use crate::commands::mon::{MAX_SPACE, spacebar};
use crate::commands::{CallbackType, CommandTree, HelpTree, exec_one};
use crate::encoder::encoder;
use crate::rn_bmp_cmd_c::{rttField_ADDRESS, rttField_ENABLED, rttField_POLLING};

crate::setup_log!(false);
crate::gdb_print_init!();
use crate::bmpwarning;
use crate::gdb_print;
use crate::rtt::{get_rtt_info, set_rtt_info};

pub const RTTSymbolName: [&str; 1] = ["_SEGGER_RTT"];
/*
 *
 */
#[unsafe(no_mangle)]
pub fn rtt_processing(key: &str, value: &str) -> bool {
    bmpwarning!("processing :key {} value {}", key, value);
    gdb_print!("Rtt Key  : {}", key);
    gdb_print!("Value Key  : {}\n", key);
    true
}

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
        command: "poll",
        min_args: 3,
        require_connected: false,
        cb: CallbackType::text(_poll),
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
        command: "poll min_ms max_ms error_limit",
        help: "set polling rate.",
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
    info.found = 0;
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
    info.found = 0;
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
    let info = get_rtt_info();
    gdb_print!("Rtt : \n");
    gdb_print!("\t**ACTIVE** :{}\n", TrueFalse(info.enabled & info.found));
    gdb_print!("\tFound      :{}\n", TrueFalse(info.found));
    gdb_print!("\tEnabled    :{}\n", TrueFalse(info.enabled));
    gdb_print!("\tFound      :{}\n", TrueFalse(info.found));
    gdb_print!("\tMinAddress :0x{:x}\n", info.min_address);
    gdb_print!("\tMaxAddress :0x{:x}\n", info.max_address);
    gdb_print!("\tCtrlAddress:0x{:x}\n", info.cb_address);
    gdb_print!("\tPollMin(ms):{}\n", info.min_poll_ms);
    gdb_print!("\tPollMax(ms):{}\n", info.max_poll_ms);
    gdb_print!("\tPollErr    :{}\n", info.max_poll_error);
    encoder::reply_ok();
    true
}
/*
 *
 *
 */
fn _scan(_command: &str, args: &[&str]) -> bool {
    let start: u32 = crate::parsing_util::ascii_hex_or_dec_to_u32(args[0]);
    let end: u32 = crate::parsing_util::ascii_hex_or_dec_to_u32(args[1]);
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
/*
 * scan ram to find the RTT block...
 * Ram BEGIN_ADDR END_ADDR
 */
fn _poll(_command: &str, args: &[&str]) -> bool {
    let min: u32 = crate::parsing_util::ascii_hex_or_dec_to_u32(args[0]);
    let max: u32 = crate::parsing_util::ascii_hex_or_dec_to_u32(args[1]);
    let er: u32 = crate::parsing_util::ascii_hex_or_dec_to_u32(args[2]);
    if max < min {
        gdb_print!("Invalid minmax\n");
        encoder::reply_e01();
        return true;
    }
    let mut info = get_rtt_info();
    info.min_poll_ms = min;
    info.max_poll_ms = max;
    info.max_poll_error = er;
    set_rtt_info(rttField_POLLING, &info);
    encoder::reply_ok();
    true
}
// EOF
