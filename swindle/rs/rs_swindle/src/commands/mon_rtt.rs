use crate::bmp;
use crate::commands::{CallbackType, CommandTree, HelpTree, exec_one};
use crate::encoder::encoder;
use numtoa::NumToA;

use crate::commands::mon::MAX_SPACE;
use crate::commands::mon::spacebar;

crate::setup_log!(false);
crate::gdb_print_init!();

use crate::gdb_print;
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
        require_connected: false,
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
        require_connected: false,
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
pub fn _help(_command: &str, _args: &[&str]) -> bool {
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
pub fn _enable(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_ok();
    true
}
/*
 *
 */
pub fn _disable(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_ok();
    true
}
/*
 *
 */
pub fn _status(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_ok();
    true
}
/*
 *
 */
pub fn _cblock(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_ok();
    true
}
/*
 *
 */
pub fn _scan(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_ok();
    true
}
// EOF
