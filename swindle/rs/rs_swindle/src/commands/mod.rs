//! GDB command dispatch — routes incoming packets to handler functions.
//!
//! Implements the main command dispatch table for the GDB remote protocol.
//! Each GDB packet is matched against a tree of known commands and dispatched
//! to the appropriate handler.
//!
//! ## Command tree
//!
//! The `main_command_tree` array defines all supported GDB commands with:
//!
//! - The command string (e.g. `"g"`, `"m"`, `"vCont"`)
//! - Minimum argument count
//! - Whether a target connection is required
//! - A callback (text or raw binary)
//! - Argument separators
//!
//! ## References
//!
//! - <https://sourceware.org/gdb/onlinedocs/gdb/Packets.html>
//! - <https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html>


use crate::bmp;
use crate::encoder::encoder;
use crate::parsing_util;
use alloc::vec::Vec;

pub mod breakpoints;
mod flash;
mod memory;
mod mon;
mod mon_ch32vxx;
mod mon_rtt;
pub mod q;
pub mod q_thread;
mod registers;
pub mod run;
pub mod symbols;
mod v;
use alloc::vec;
use breakpoints::_Z;
use breakpoints::_z;
use flash::_flashv;
use memory::{_X, _m};
use q::_q;
use registers::{_P, _g, _p};
use v::_v;

use run::{_R, _c, _k, _s, _vCont};

type Callback_raw = fn(command: &[u8]) -> bool;
type Callback_text = fn(command: &str, args: &[&str]) -> bool;

setup_log!(false);
crate::gdb_print_init!();
//use crate::{bmplog, gdb_print};

/// Type of command callback: text-split or raw binary.
pub enum CallbackType {
    text(Callback_text),
    raw(Callback_raw),
}
//
//#[derive(Sized)]
/// A single entry in the GDB command dispatch tree.
pub struct CommandTree {
    command: &'static str,
    min_args: usize,
    require_connected: bool,
    cb: CallbackType, // string + strings
    // this is the separator (if any) after the command
    // for example for vCont:A, it is ':'
    start_separator: u8,
    // this is the separator between next args
    // for example vCont:A,B,C it is ','
    next_separator: u8,
}
//
/// A help entry mapping a command to its description.
pub struct HelpTree {
    command: &'static str,
    help: &'static str,
}
//
const main_command_tree: [CommandTree; 23] = [
    CommandTree {
        command: "QStartNoAckMode",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_qStartNoAckMode),
        start_separator: 0,
        next_separator: 0,
    },
    CommandTree {
        command: "!",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_extendedMode),
        start_separator: 0,
        next_separator: 0,
    }, // enable extended mode
    CommandTree {
        command: "T",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(q_thread::_T),
        start_separator: 0,
        next_separator: 0,
    }, // enable extended mode
    CommandTree {
        command: "Hg",
        min_args: 1,
        require_connected: true,
        cb: CallbackType::text(q_thread::_Hg),
        start_separator: 0,
        next_separator: 0,
    }, // select thread
    CommandTree {
        command: "Hc",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_Hc),
        start_separator: 0,
        next_separator: 0,
    }, //
    CommandTree {
        command: "vCont", // vCont;c
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_vCont),
        start_separator: b';',
        next_separator: b';',
    },
    CommandTree {
        command: "vFlash",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::raw(_flashv),
        start_separator: 0,
        next_separator: 0,
    }, // test
    CommandTree {
        command: "v",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::raw(_v),
        start_separator: 0,
        next_separator: 0,
    }, // test
    CommandTree {
        command: "q",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::raw(_q),
        start_separator: 0,
        next_separator: 0,
    }, // see q commands in commands/q.rs
    CommandTree {
        command: "g",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_g),
        start_separator: 0,
        next_separator: 0,
    }, // read registers
    CommandTree {
        command: "?",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_mark),
        start_separator: 0,
        next_separator: 0,
    }, // reason for halt
    CommandTree {
        command: "X",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::raw(_X), // addr,length:XX…’  X2000000;4:4
        start_separator: 0,
        next_separator: 0,
    }, // write binary
    CommandTree {
        command: "m",
        min_args: 2,
        require_connected: true,
        cb: CallbackType::text(_m), // m2000000,4
        start_separator: 0,
        next_separator: b',',
    }, // read memory
    CommandTree {
        command: "p",
        min_args: 1,
        require_connected: true,
        cb: CallbackType::text(_p),
        start_separator: 0,
        next_separator: 0,
    }, // read register
    CommandTree {
        // Pf=123
        command: "P",
        min_args: 2,
        require_connected: true,
        cb: CallbackType::text(_P),
        start_separator: 0,
        next_separator: b'=',
    }, // write register
    CommandTree {
        command: "z",
        min_args: 2,
        require_connected: true,
        cb: CallbackType::text(_z), // breakpoint (remove) ‘z type,addr,kind’
        start_separator: 0,
        next_separator: b',',
    }, // read memory
    CommandTree {
        command: "Z",
        min_args: 2,
        require_connected: true,
        cb: CallbackType::text(_Z), // breakpoint (set) '‘Z type,addr,kind’'
        start_separator: 0,
        next_separator: b',',
    },
    CommandTree {
        command: "R",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_R), // restart
        start_separator: 0,
        next_separator: 0,
    },
    CommandTree {
        command: "r",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_R), // reset the whole system
        start_separator: 0,
        next_separator: 0,
    },
    CommandTree {
        command: "s",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_s), // single step
        start_separator: 0,
        next_separator: 0,
    },
    CommandTree {
        command: "k",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_k), // kill
        start_separator: 0,
        next_separator: 0,
    },
    CommandTree {
        command: "c",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_c), // resume c[addr]
        start_separator: 0,
        next_separator: 0,
    },
    CommandTree {
        command: "D",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_D), // detach
        start_separator: 0,
        next_separator: 0,
    },
];
/// Execute a single command by matching it against a dispatch tree.
///
/// Returns `true` if the command was recognised and handled.
#[unsafe(no_mangle)]
pub fn exec_one(tree: &[CommandTree], command: &[u8]) -> bool {
    let connected: bool = bmp::bmp_attached();
    for c in tree {
        if command.starts_with(c.command.as_bytes()) {
            if !connected && c.require_connected {
                gdb_print!(
                    "The following command  cannot be used while not connected :<",
                    c.command
                );
                gdb_print!(">\n");
                bmplog!("Command not ok while not connected {} \n", c.command);
                encoder::reply_e01();
                return true;
            } else {
                return match c.cb {
                    CallbackType::text(y) => {
                        let as_string = unsafe { core::str::from_utf8_unchecked(command) };
                        let prefix_size = c.command.len() + if c.start_separator != 0 { 1 } else { 0 };
                        let mut conf: Vec<&str>;
                        if as_string.len() > prefix_size && c.next_separator != 0 {
                            conf = as_string[prefix_size..].split(c.next_separator as char).collect();
                        } else {
                            if as_string.len() > prefix_size {
                                conf = vec![&as_string[prefix_size..]];
                            } else {
                                conf = vec![];
                            }
                        }
                        for i in conf.iter_mut() {
                            *i = parsing_util::chomp(i);
                        }
                        if conf.len() < c.min_args {
                            bmplog!("Wrong number of parameters\n");
                            return false;
                        }
                        (y)(as_string, &conf)
                    }
                    CallbackType::raw(x) => {
                        (x)(command)
                    }
                };
            }
        }
    }
    false
}

/// Main entry point: dispatch a GDB command string to its handler.
///
/// If the command is not recognised, sends an empty reply (unsupported).
#[unsafe(no_mangle)]
pub fn exec(command: &[u8]) {
    if !exec_one(&main_command_tree, command) {
        encoder::simple_send(""); // unsupported
        bmplog!("Unsupported cmd\n");
    }
}
//
//
/// Handle the extended mode `!` command — always OK.
fn _extendedMode(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_ok();
    true
}
// select thread
/// Handle `Hc` (set thread for continue) — not supported, returns E01.
fn _Hc(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_e01();
    true
}

// detach
/// Handle `D` (detach) — detach from target and reply OK.
fn _D(_command: &str, _args: &[&str]) -> bool {
    if bmp::bmp_attached() {
        bmp::bmp_detach();
    }
    encoder::reply_ok();
    true
}

//
// Request reason for halt
/// Handle `?` (reason for halt) — always reports `W00` (exited).
fn _mark(_command: &str, _args: &[&str]) -> bool {
    //NOTARGET
    encoder::simple_send("W00");
    true
}
// used by lldb
/// Handle `QStartNoAckMode` — always OK (used by LLDB).
fn _qStartNoAckMode(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_ok();
    true
}
// EOF
