//! RTT monitor sub-commands (`mon rtt`).
//!
//! Provides the `mon rtt` sub-command tree for controlling SEGGER RTT
//! (Real-Time Transfer) on the target:
//!
//! - `mon rtt enable` — enable RTT polling
//! - `mon rtt disable` — disable RTT polling
//! - `mon rtt status` — show current RTT state
//! - `mon rtt help` — show RTT help

use crate::commands::mon::{MAX_SPACE, spacebar};
use crate::commands::{CallbackType, CommandTree, HelpTree, exec_one};
use crate::encoder::encoder;
use crate::parsing_util;
use crate::setting_keys::RTT_SETTING_KEY;

setup_log!(false);
crate::gdb_print_init!();
//use crate::bmpwarning;
//use crate::gdb_print;

use crate::settings;

/// Clear the stored RTT control block address from settings.
pub fn rtt_clear_symbols() -> bool {
    settings::remove(RTT_SETTING_KEY);
    true
}
/*
 *
 */
/// Process an RTT symbol from the target's ELF symbol table.
///
/// Stores the RTT control block address in persistent settings.
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
        start_separator: b' ',
        next_separator: b' ',
    },
    CommandTree {
        command: "enable",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_enable),
        start_separator: b' ',
        next_separator: b' ',
    },
    CommandTree {
        command: "disable",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_disable),
        start_separator: b' ',
        next_separator: b' ',
    },
    CommandTree {
        command: "status",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_status),
        start_separator: b' ',
        next_separator: b' ',
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
/// Dispatch `mon rtt` sub-commands.
#[unsafe(no_mangle)]
pub fn _rtt(_command: &str, args: &[&str]) -> bool {
    let as_string = args[0..].join(" ");
    exec_one(&rtt_command_tree, as_string.as_bytes())
}

/*
 *
 */
/// Show RTT help.
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
        gdb_print!("mon rtt  ", &cmd);
        gdb_print!(" ", &pad);
        gdb_println!(": ", &(i.help));
    }
    encoder::reply_ok();
    true
}
/*
 *
 */
/// Enable RTT polling on the target.
fn _enable(_command: &str, _args: &[&str]) -> bool {
    crate::rtt::swindle_enable_rtt(true);
    encoder::reply_ok();
    true
}
/*
 *
 */
/// Disable RTT polling on the target.
fn _disable(_command: &str, _args: &[&str]) -> bool {
    crate::rtt::swindle_enable_rtt(false);
    encoder::reply_ok();
    true
}
/*
 *
 */
#[allow(dead_code)]
fn TrueFalse(onoff: u32) -> &'static str {
    if onoff == 0 { "false" } else { "true" }
}
/*
 *
 */
/// Show current RTT status (enabled/disabled, control block address).
fn _status(_command: &str, _args: &[&str]) -> bool {
    crate::rtt::swindle_rtt_print_info();
    encoder::reply_ok();
    true
}
// EOF
