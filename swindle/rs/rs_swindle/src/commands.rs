// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use crate::encoder::encoder;
use alloc::vec::Vec;

pub mod breakpoints;
mod flash;
mod memory;
mod mon;
pub mod q;
pub mod q_thread;
mod registers;
pub mod run;
mod v;

use breakpoints::_z;
use breakpoints::_Z;
use flash::_flashv;
use memory::{_m, _X};
use q::_q;
use registers::{_g, _p, _P};
use v::_v;

use run::{_c, _k, _s, _vCont, _R};

type Callback_raw = fn(command: &str, args: &[u8]) -> bool;
type Callback_text = fn(command: &str, args: &[&str]) -> bool;

crate::setup_log!(true);
crate::gdb_print_init!();
use crate::{bmplog, bmpwarning, gdb_print};

pub enum CallbackType {
    text(Callback_text),
    raw(Callback_raw),
}

pub struct CommandTree {
    command: &'static str,
    args: usize,
    require_connected: bool,
    cb: CallbackType, // string + strings
}

const main_command_tree: [CommandTree; 22] = [
    CommandTree {
        command: "!",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_extendedMode),
    }, // enable extended mode
    CommandTree {
        command: "T",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(q_thread::_T),
    }, // enable extended mode
    CommandTree {
        command: "Hg",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(q_thread::_Hg),
    }, // select thread
    CommandTree {
        command: "Hc",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_Hc),
    }, //
    CommandTree {
        command: "vCont",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_vCont),
    },
    CommandTree {
        command: "vFlash",
        args: 0,
        require_connected: true,
        cb: CallbackType::raw(_flashv),
    }, // test
    CommandTree {
        command: "v",
        args: 0,
        require_connected: false,
        cb: CallbackType::raw(_v),
    }, // test
    CommandTree {
        command: "q",
        args: 0,
        require_connected: false,
        cb: CallbackType::raw(_q),
    }, // see q commands in commands/q.rs
    CommandTree {
        command: "g",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_g),
    }, // read registers
    CommandTree {
        command: "?",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_mark),
    }, // reason for halt
    CommandTree {
        command: "X",
        args: 0,
        require_connected: true,
        cb: CallbackType::raw(_X),
    }, // write binary
    CommandTree {
        command: "m",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_m),
    }, // read memory
    CommandTree {
        command: "p",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_p),
    }, // read register
    CommandTree {
        command: "P",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_P),
    }, // write register
    CommandTree {
        command: "z",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_z),
    }, // read memory
    CommandTree {
        command: "Z",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_Z),
    },
    CommandTree {
        command: "R",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_R),
    },
    CommandTree {
        command: "r",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_R),
    },
    CommandTree {
        command: "s",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_s),
    },
    CommandTree {
        command: "k",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_k),
    },
    CommandTree {
        command: "c",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_c),
    },
    CommandTree {
        command: "D",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_D),
    },
];

pub fn exec_one(tree: &[CommandTree], command: &str, args: &[u8]) -> bool {
    let connected: bool = crate::bmp::bmp_attached();
    bmplog!(command);
    bmplog!("\n");
    let empty: &str = "";
    for c in tree {
        // let c = &tree[i];
        if command.starts_with(c.command)
        // if the expected command begins with the receive command..
        {
            if !connected && c.require_connected {
                gdb_print!("Command {} cannot be used while not connected\n", c.command);
                bmplog!("Command not ok while not connected {} \n", c.command);
                encoder::reply_e01();
                return true;
            } else {
                // Is it a regular callback or binary callback
                return match c.cb {
                    CallbackType::text(y) => {
                        // split args by ":"
                        let as_string: &str = match core::str::from_utf8(args) {
                            Ok(x) => x,
                            Err(_x) => empty,
                        };
                        let conf: Vec<&str> = as_string.split(':').collect();
                        (y)(command, &conf)
                    }
                    CallbackType::raw(x) => (x)(command, args),
                };
                // return (c.cb)(command, args);
            }
        }
    }
    false
}

pub fn exec(command: &str, args: &[u8]) {
    if !exec_one(&main_command_tree, command, args) {
        {
            encoder::simple_send(""); // unsupported
            bmplog!("Unsupported cmd :{} \n", command);
            gdb_print!("!!!Command {} is not supported!!\n", command);
        }
    }
}
//
//
fn _extendedMode(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_ok();
    true
}
// select thread
fn _Hc(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_e01();
    true
}

// detach
fn _D(_command: &str, _args: &[&str]) -> bool {
    if crate::bmp::bmp_attached() {
        crate::bmp::bmp_detach();
    }
    encoder::reply_ok();
    true
}

//
// Request reason for halt
fn _mark(_command: &str, _args: &[&str]) -> bool {
    //NOTARGET
    encoder::simple_send("W00");
    true
}

// EOF
