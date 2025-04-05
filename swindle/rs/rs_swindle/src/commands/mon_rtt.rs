use crate::bmp;
use crate::commands::mon::get_enable_reset;
use crate::commands::{CallbackType, CommandTree, HelpTree, exec_one};
use crate::encoder::encoder;
use numtoa::NumToA;

pub const MAX_SPACE: usize = 40;
pub const spacebar: [u8; MAX_SPACE] = [32; MAX_SPACE];

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
        command: "bmp",
        help: "Forward the command to bmp mon command.\n\tExample : mon bmp mass_erase is the same as mon mass_erase on a bmp..",
    },
    HelpTree {
        command: "boards",
        help: "Display supported boards. This is set at build time.",
    },
];
/*
 *
 */
pub fn _rtt(_command: &str, args: &[&str]) -> bool {
    /*let len = args.len();
    if len > 0 {
        // ok we have an input
        let ws = ascii_string_decimal_to_u32(args[0]);
        bmp::bmp_set_wait_state(ws);
    }
    let w: u32 = bmp::bmp_get_wait_state();
    gdb_print!("wait states are now {}\n", w);
    */
    encoder::reply_ok();
    true
}

/*
 *
 */
pub fn _help(_command: &str, args: &[&str]) -> bool {
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
        gdb_print!("mon {}{} : {}\n", &cmd, &pad, &(i.help));
    }
    true
}
/*
 *
 */
pub fn _two(_command: &str, args: &[&str]) -> bool {
    true
}
/*
 *
 */
pub fn _tree(_command: &str, args: &[&str]) -> bool {
    true
}
// EOF
