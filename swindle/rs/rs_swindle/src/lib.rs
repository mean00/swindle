#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]
#![allow(static_mut_refs)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]
//
#[macro_use]
extern crate alloc;
//#![allow(unused_imports)]
mod bmp;
mod bmplogger;
mod commands;
mod crc;
mod decoder;
mod encoder;
mod freertos;
mod glue;
mod packet_symbols;
mod parsing_util;
mod rn_bmp_cmd_c;
mod rtt_consts;
mod setting_keys;
mod settings;
//mod rpc;
pub mod rpc_common;
#[cfg(feature = "hosted")]
pub mod rpc_host;
#[cfg(not(feature = "hosted"))]
pub mod rpc_target;
mod rtt;
mod sw_breakpoints;
mod util;

use crate::commands::run;
use crate::decoder::gdb_stream;
use core::mem::MaybeUninit;
use decoder::RESULT_AUTOMATON;
use packet_symbols::{CHAR_ACK, CHAR_NACK, INPUT_BUFFER_SIZE};
crate::gdb_print_init!();

crate::setup_log!(false);
//use crate::{bmplog,bmpwarning};

static mut autoauto: MaybeUninit<gdb_stream<INPUT_BUFFER_SIZE>> = MaybeUninit::uninit();
/*
 *
 */
fn get_autoauto() -> &'static mut gdb_stream<INPUT_BUFFER_SIZE> {
    unsafe { autoauto.assume_init_mut() }
}
fn clear_autoauto() {
    get_autoauto().set_available(false);
}
#[unsafe(no_mangle)]
pub extern "C" fn rngdbstub_init() {
    settings::init_settings();
    unsafe {
        autoauto.write(gdb_stream::<INPUT_BUFFER_SIZE>::new());
        get_autoauto().set_available(true);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn rngdbstub_shutdown() {
    bmp::bmp_detach();
    clear_autoauto();
}
/*
 *
 */
unsafe extern "C" {
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
/*
 *
 *
 */
#[unsafe(no_mangle)]
pub extern "C" fn rngdbstub_run(l: usize, d: *const cty::c_uchar) {
    let mut data_as_slice: &[u8];
    unsafe {
        data_as_slice = core::slice::from_raw_parts(d, l);
    }
    // The target is running, the only valid thing we are expecting is 3 or 4 (i.e. stop request)
    // i'm not sure what happens escaping-wise if we let the parser handle it
    if run::target_is_running() {
        if data_as_slice.len() == 1 {
            match data_as_slice[0] {
                3 => run::target_halt(),
                _ => bmplog!("Warning : garbage received"),
            }
        }
        return;
    }
    // the target is stopped
    // we can parse the incoming commands
    let available = get_autoauto().get_available();
    match available {
        true => {
            while !data_as_slice.is_empty() {
                let x = get_autoauto();
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
                            #[cfg(not(feature = "hosted"))]
                            rpc_target::rpc(s);
                            bmplog!("Rpc done\n");
                        }
                    }
                    RESULT_AUTOMATON::Ready => {
                        // ok we have a full string...
                        bmplog!("Gdb call\n");
                        let s = x.get_result();
                        if s.is_empty() {
                            bmplog!("Cannot read string");
                        } else {
                            bmplog!("--> ACK\n");
                            rngdb_send_data_u8(&[CHAR_ACK]);
                            rngdb_output_flush();
                            let as_string: &str;
                            unsafe {
                                as_string = core::str::from_utf8_unchecked(s);
                            }
                            bmplog!("Exec..:");
                            bmplog!(as_string);
                            bmplog!("\n");
                            if bmp::bmp_try() {
                                commands::exec(as_string);
                                bmplog!("Exec done\n");
                            }
                            if bmp::bmp_catch() != 0 {
                                gdb_print!("Stray exception\n");
                                encoder::encoder::reply_e01();
                            }
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
        false => panic!("noauto"),
    };
}

// EOF
