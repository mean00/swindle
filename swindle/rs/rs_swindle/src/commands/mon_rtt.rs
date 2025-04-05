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
const rtt_command_tree: [CommandTree; 3] = [
    CommandTree {
        command: "help",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_help),
        start_separator: " ",
        next_separator: " ",
    },
    CommandTree {
        command: "two",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(_two),
        start_separator: " ",
        next_separator: " ",
    },
    CommandTree {
        command: "tree",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_tree),
        start_separator: " ",
        next_separator: " ",
    },
];
/*
 *
 */
const rtt_help_tree: [HelpTree; 3] = [
    HelpTree {
        command: "help",
        help: "Display help.",
    },
    HelpTree {
        command: "rttcmd1",
        help: "rttcmd1 ..",
    },
    HelpTree {
        command: "rttcmd2",
        help: "rttcmd2 .",
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
pub fn _two(_command: &str, _args: &[&str]) -> bool {
    true
}
/*
 *
 */
pub fn _tree(_command: &str, _args: &[&str]) -> bool {
    true
}
// EOF
