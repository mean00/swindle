use crate::commands::mon::{MAX_SPACE, spacebar};
use crate::commands::{CallbackType, CommandTree, HelpTree, exec_one};
use crate::encoder::encoder;
use crate::parsing_util;
use crate::rtt_consts::*;

crate::setup_log!(false);
crate::gdb_print_init!();
use crate::bmpwarning;
use crate::gdb_print;

use crate::settings;
/*
 *
 *
 */
pub fn rtt_clear_symbols() -> bool {
    settings::remove(RTT_SETTING_KEY);
    true
}
/*
 *
 */
pub fn rtt_processing(key: &str, value_str: &str) -> bool {
    bmpwarning!("processing :key {} value {}", key, value_str);
    let value = parsing_util::ascii_hex_to_u32(value_str);
    settings::set(RTT_SETTING_KEY, value);
    true
}

/*
 *
 */
const rtt_command_tree: [CommandTree; 4] = [
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
];
/*
 *
 */
const rtt_help_tree: [HelpTree; 4] = [
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
    crate::rtt::swindle_enable_rtt(true);
    encoder::reply_ok();
    true
}
/*
 *
 */
fn _disable(_command: &str, _args: &[&str]) -> bool {
    crate::rtt::swindle_enable_rtt(false);
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
    crate::rtt::swindle_rtt_print_info();
    encoder::reply_ok();
    true
}
// EOF
