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
//use crate::{bmplog, gdb_print};

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
    start_separator: u8,
    // this is the separator between next args
    // for example vCont:A,B,C it is ','
    next_separator: u8,
}
//
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
#[unsafe(no_mangle)]
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
                gdb_print!(
                    "The following command  cannot be used while not connected :<",
                    c.command
                );
                gdb_print!(">\n");
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
                        let prefix_size = c.command.len() + if c.start_separator != 0 { 1 } else { 0 };
                        let mut conf: Vec<&str>;
                        if as_string.len() > prefix_size && c.next_separator != 0 {
                            //} && !c.next_separator.is_empty() {
                            conf = as_string[prefix_size..].split(c.next_separator as char).collect();
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
                        let prefix_size = c.command.len() + if c.start_separator != 0 { 1 } else { 0 };
                        if command.len() > prefix_size && c.next_separator != 0 {
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

#[unsafe(no_mangle)]
pub fn exec(command: &str) {
    let args: &[u8] = &[];
    if !exec_one(&main_command_tree, command, args) {
        {
            encoder::simple_send(""); // unsupported
            bmplog!("Unsupported cmd :{} \n", command);
            gdb_print!("!!! The following command  is not supported : <", command);
            gdb_print!(">\n");
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
// used by lldb
fn _qStartNoAckMode(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_ok();
    true
}
// EOF
