// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

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

type Callback_raw = fn(command: &str, args: &[u8]) -> bool;
type Callback_text = fn(command: &str, args: &[&str]) -> bool;

crate::setup_log!(false);
crate::gdb_print_init!();
use crate::{bmplog, gdb_print};

pub enum CallbackType {
    text(Callback_text),
    raw(Callback_raw),
}
//
//#[derive(Sized)]
pub struct CommandTree {
    command: &'static str,
    min_args: usize,
    require_connected: bool,
    cb: CallbackType, // string + strings
    // this is the separator (if any) after the command
    // for example for vCont:A, it is ':'
    start_separator: &'static str,
    // this is the separator between next args
    // for example vCont:A,B,C it is ','
    next_separator: &'static str,
}
//
pub struct HelpTree {
    command: &'static str,
    help: &'static str,
}
//
const main_command_tree: [CommandTree; 22] = [
    CommandTree {
        command: "!",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_extendedMode),
        start_separator: "",
        next_separator: "",
    }, // enable extended mode
    CommandTree {
        command: "T",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(q_thread::_T),
        start_separator: "",
        next_separator: "",
    }, // enable extended mode
    CommandTree {
        command: "Hg",
        min_args: 1,
        require_connected: true,
        cb: CallbackType::text(q_thread::_Hg),
        start_separator: "",
        next_separator: "",
    }, // select thread
    CommandTree {
        command: "Hc",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_Hc),
        start_separator: "",
        next_separator: "",
    }, //
    CommandTree {
        command: "vCont", // vCont;c
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_vCont),
        start_separator: ";",
        next_separator: ";",
    },
    CommandTree {
        command: "vFlash",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::raw(_flashv),
        start_separator: "",
        next_separator: "",
    }, // test
    CommandTree {
        command: "v",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::raw(_v),
        start_separator: "",
        next_separator: "",
    }, // test
    CommandTree {
        command: "q",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::raw(_q),
        start_separator: "",
        next_separator: "",
    }, // see q commands in commands/q.rs
    CommandTree {
        command: "g",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_g),
        start_separator: "",
        next_separator: "",
    }, // read registers
    CommandTree {
        command: "?",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_mark),
        start_separator: "",
        next_separator: "",
    }, // reason for halt
    CommandTree {
        command: "X",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::raw(_X), // addr,length:XX…’  X2000000;4:4
        start_separator: "",
        next_separator: "",
    }, // write binary
    CommandTree {
        command: "m",
        min_args: 2,
        require_connected: true,
        cb: CallbackType::text(_m), // m2000000,4
        start_separator: "",
        next_separator: ",",
    }, // read memory
    CommandTree {
        command: "p",
        min_args: 1,
        require_connected: true,
        cb: CallbackType::text(_p),
        start_separator: " ",
        next_separator: "",
    }, // read register
    CommandTree {
        // Pf=123
        command: "P",
        min_args: 2,
        require_connected: true,
        cb: CallbackType::text(_P),
        start_separator: "",
        next_separator: "=",
    }, // write register
    CommandTree {
        command: "z",
        min_args: 2,
        require_connected: true,
        cb: CallbackType::text(_z), // breakpoint (remove) ‘z type,addr,kind’
        start_separator: "",
        next_separator: ",",
    }, // read memory
    CommandTree {
        command: "Z",
        min_args: 2,
        require_connected: true,
        cb: CallbackType::text(_Z), // breakpoint (set) '‘Z type,addr,kind’'
        start_separator: "",
        next_separator: ",",
    },
    CommandTree {
        command: "R",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_R), // restart
        start_separator: "",
        next_separator: "",
    },
    CommandTree {
        command: "r",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_R), // reset the whole system
        start_separator: "",
        next_separator: "",
    },
    CommandTree {
        command: "s",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_s), // single step
        start_separator: "",
        next_separator: "",
    },
    CommandTree {
        command: "k",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_k), // kill
        start_separator: "",
        next_separator: "",
    },
    CommandTree {
        command: "c",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_c), // resume c[addr]
        start_separator: "",
        next_separator: "",
    },
    CommandTree {
        command: "D",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_D), // detach
        start_separator: "",
        next_separator: "",
    },
];

pub fn exec_one(tree: &[CommandTree], command: &str, _args: &[u8]) -> bool {
    let connected: bool = bmp::bmp_attached();
    //bmplog!(command);
    //bmplog!("\n");
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
                        // split args by splitter
                        let as_string = command;
                        // do we have a start separator ?
                        let prefix_size = c.command.len() + c.start_separator.len();
                        let mut conf: Vec<&str>;
                        if as_string.len() > prefix_size && !c.next_separator.is_empty() {
                            //} && !c.next_separator.is_empty() {
                            conf = as_string[prefix_size..].split(c.next_separator).collect();
                        } else {
                            // no extra data
                            bmplog!("command : {} \n", command);
                            if as_string.len() > prefix_size {
                                conf = vec![&as_string[prefix_size..]];
                            } else {
                                conf = vec![];
                            }
                        }
                        // Remove starting " "
                        for i in conf.iter_mut() {
                            *i = parsing_util::chomp(i);
                        }
                        bmplog!(command);
                        bmplog!("\n");
                        bmplog!("unpacked command : <{}> \n", command);
                        for i in &conf {
                            bmplog!("\t<{}>\n", i);
                        }
                        bmplog!("\n");
                        if conf.len() < c.min_args {
                            bmplog!("Wrong number of parameters\n");
                            return false;
                        }
                        (y)(command, &conf)
                    }
                    CallbackType::raw(x) => {
                        bmplog!(c.command);
                        bmplog!("\n");
                        let prefix_size = c.command.len() + c.start_separator.len();
                        if command.len() > prefix_size && !c.next_separator.is_empty() {
                            return (x)(command, &command.as_bytes()[prefix_size..]);
                        } else {
                            // no extra data
                            return (x)(command, &[]);
                        }
                    }
                };
                // return (c.cb)(command, args);
            }
        }
    }
    false
}

pub fn exec(command: &str) {
    let args: &[u8] = &[];
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
    if bmp::bmp_attached() {
        bmp::bmp_detach();
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
