#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

mod bmp;
mod bmplogger;
mod commands;
mod decoder;
mod encoder;
mod glue;
mod packet_symbols;
mod parsing_util;
mod poppingbuffer;
mod rn_bmp_cmd_c;
mod util;

use crate::decoder::gdb_stream;
use packet_symbols::{CHAR_ACK, CHAR_NACK, INPUT_BUFFER_SIZE};
extern crate alloc;
use alloc::vec::Vec;
use decoder::RESULT_AUTOMATON;
use numtoa::NumToA;

crate::setup_log!(false);
//use crate::{bmplog,bmpwarning};

//

#[no_mangle]

static mut autoauto: Option<gdb_stream<INPUT_BUFFER_SIZE>> = None;

#[no_mangle]
extern "C" fn rngdbstub_init() {
    unsafe {
        if autoauto.is_some() {
            //panic!("notnull");
            autoauto = None;
        }
        autoauto = Some(gdb_stream::<INPUT_BUFFER_SIZE>::new());
    }
}
#[no_mangle]
extern "C" fn rngdbstub_shutdown() {
    unsafe {
        if autoauto.is_none() {
            //            panic!("notsome");
        }
        autoauto = None;
    }
}
/*
 *
 */
extern "C" {
    fn rngdb_send_data_c(sz: u32, ptr: *const cty::c_uchar);
    fn rngdb_output_flush_c();
}
/*
 */
fn rngdb_output_flush() {
    unsafe {
        rngdb_output_flush_c();
    }
}
/*
 */
fn rngdb_send_data(data: &str) {
    unsafe {
        rngdb_send_data_c(data.len() as u32, data.as_ptr());
    }
}
/*
 */
fn rngdb_send_data_u8(data: &[u8]) {
    unsafe {
        rngdb_send_data_c(data.len() as u32, data.as_ptr());
    }
}
/**
 *
 *
 */
#[no_mangle]
extern "C" fn rngdbstub_run(l: usize, d: *const cty::c_uchar) {
    unsafe {
        let mut data_as_slice: &[u8] = core::slice::from_raw_parts(d, l);
        let empty1: [u8; 0] = [0; 0];
        let empty: &[u8] = &empty1;

        // The target is running, the only valid thing we are expecting is 3 or 4 (i.e. stop request)
        // i'm not sure what happens escaping-wise if we let the parser handle it
        if crate::commands::run::target_is_running() {
            if data_as_slice.len() == 1 {
                match data_as_slice[0] {
                    3 => crate::commands::run::target_halt(),
                    _ => bmplog!("Warning : garbage received"),
                }
            }
            return;
        }
        // the target is stopped
        // we can parse the incoming commands
        match autoauto {
            Some(ref mut x) => {
                while !data_as_slice.is_empty() {
                    let consumed: usize;
                    let state: RESULT_AUTOMATON;
                    bmplog!("Parsing..\n");
                    (consumed, state) = x.parse(data_as_slice);
                    bmplog!("Parsed..\n");
                    match state {
                        RESULT_AUTOMATON::RpcReady => {
                            bmplog!("Rpc....\n");
                            let s = x.get_result(); // s is a RPC command block
                            if !s.is_empty() {
                                //bmplog!("--> ACK\n");
                                //rngdb_send_data( CHAR_ACK );
                                commands::rpc::rpc(s);
                                bmplog!("Rpc done\n");
                            }
                        }
                        RESULT_AUTOMATON::Ready => {
                            // ok we have a full string...
                            bmplog!("Gdb call\n");
                            let s = x.get_result();
                            let command: &[u8];
                            let args: &[u8];
                            match crate::parsing_util::split_command(s) {
                                None => {
                                    bmplog!("Cannot convert string");
                                    command = empty;
                                    args = empty;
                                }
                                Some((x, y)) => {
                                    command = x;
                                    args = y;
                                }
                            }
                            if command.is_empty() {
                                bmplog!("Cannot read string");
                            } else {
                                bmplog!("--> ACK\n");
                                rngdb_send_data_u8(&[CHAR_ACK]);
                                rngdb_output_flush();
                                let as_string = core::str::from_utf8_unchecked(command);
                                bmplog!("Exec..:");
                                bmplog!(as_string);
                                bmplog!("\n");
                                commands::exec(as_string, args);
                                bmplog!("Exec done\n");
                            }
                        }
                        RESULT_AUTOMATON::Error => {
                            rngdb_send_data_u8(&[CHAR_NACK]);
                            rngdb_output_flush();
                        }
                        RESULT_AUTOMATON::Continue => (),
                        RESULT_AUTOMATON::Reset => (),
                    }
                    data_as_slice = &data_as_slice[consumed..];
                }
            }
            None => panic!("noauto"),
        };
    }
}

// EOF
