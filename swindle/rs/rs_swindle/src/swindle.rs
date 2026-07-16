//! Main GDB stub loop — packet parsing and command dispatch.
//!
//! Implements the core GDB remote protocol loop: receives packets from the
//! USB/serial transport, decodes them via the state machine in `decoder`,
//! dispatches to the appropriate command handler, and sends responses back.
//!
//! ## Entry points (C-compatible)
//!
//! - `rngdbstub_init()`: Initialise the stub and settings.
//! - `rngdbstub_run()`: Process incoming GDB data (called from C ISR/loop).
//! - `rngdbstub_shutdown()`: Detach target and clean up.
//!
//! ## Flow
//!
//! 1. `rngdbstub_run()` receives raw bytes from the transport layer.
//! 2. The `gdb_stream` state machine (`decoder`) parses GDB packets.
//! 3. RPC packets are dispatched to `native::rpc_target_generated::rpc_dispatch()`.
//! 4. GDB command packets are dispatched to `commands::exec()`.
//! 5. Responses are sent back via `encoder` and the transport layer.

#![allow(static_mut_refs)]
//
use crate::bmp;
use crate::commands;
use crate::commands::run;
use crate::decoder::RESULT_AUTOMATON;
use crate::decoder::gdb_stream;
use crate::encoder;
#[cfg(not(feature = "hosted"))]
use crate::native;
use crate::packet_symbols::{CHAR_ACK, CHAR_NACK, INPUT_BUFFER_SIZE};
use crate::settings;

//
#[cfg(any(feature = "hosted", feature = "network"))]
use crate::not_usb;
#[cfg(all(not(feature = "hosted"), not(feature = "network")))]
use crate::usb;
//
#[cfg(all(not(feature = "hosted"), not(feature = "network")))]
pub(crate) fn rngdb_output_flush() {
    usb::rngdb_output_flush()
}
#[cfg(all(not(feature = "hosted"), not(feature = "network")))]
pub(crate) fn rngdb_send_data(data: &str) {
    usb::rngdb_send_data(data)
}
#[cfg(all(not(feature = "hosted"), not(feature = "network")))]
pub(crate) fn rngdb_send_data_u8(data: &[u8]) {
    usb::rngdb_send_data_u8(data)
}
#[cfg(any(feature = "hosted", feature = "network"))]
pub(crate) fn rngdb_output_flush() {
    not_usb::rngdb_output_flush()
}
#[cfg(any(feature = "hosted", feature = "network"))]
#[allow(dead_code)]
pub(crate) fn rngdb_send_data(data: &str) {
    not_usb::rngdb_send_data(data)
}
#[cfg(any(feature = "hosted", feature = "network"))]
pub(crate) fn rngdb_send_data_u8(data: &[u8]) {
    not_usb::rngdb_send_data_u8(data)
}
//
setup_log!(false);
//use crate::{bmplog,bmpwarning};

static mut autoauto: gdb_stream<INPUT_BUFFER_SIZE> = gdb_stream::new();
/*
 *
 */
fn get_autoauto() -> &'static mut gdb_stream<INPUT_BUFFER_SIZE> {
    unsafe { &mut autoauto }
}
#[unsafe(no_mangle)]
pub extern "C" fn rngdbstub_init() {
    settings::init_settings();
    unsafe {
        autoauto.init();
        get_autoauto().set_ready();
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn rngdbstub_shutdown() {
    bmp::bmp_detach();
    get_autoauto().set_not_ready();
}
/// # Safety
/// the pointer is expected to be valid !
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rngdbstub_run(l: usize, d: *const cty::c_uchar) {
    let data_as_slice: &[u8] = unsafe { core::slice::from_raw_parts(d, l) };
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
    get_autoauto().check();
    if !get_autoauto().get_available() {
        return;
    }
    run_parser(data_as_slice);
    // We can switch state here
    get_autoauto().check();
}
//
//
//
fn run_parser(data_as_slice: &[u8]) {
    let mut data_as_slice: &[u8] = data_as_slice;
    let engine = get_autoauto();
    while !data_as_slice.is_empty() {
        let consumed: usize;
        let state: RESULT_AUTOMATON;
        bmplog!("Parsing..\n");
        (consumed, state) = engine.parse(data_as_slice);
        bmplog!("Parsed..\n");
        // did we reach an end state in the state machine ?
        match state {
            RESULT_AUTOMATON::RpcReady => {
                bmplog!("Rpc....\n");
                let s = engine.get_result(); // s is a RPC command block
                if !s.is_empty() {
                    //bmplog!("--> ACK\n");
                    //rngdb_send_data( CHAR_ACK );
                    #[cfg(not(feature = "hosted"))]
                    native::rpc_target_generated::rpc_dispatch(s);
                    bmplog!("Rpc done\n");
                }
            }
            RESULT_AUTOMATON::Ready => {
                // ok we have a full string...
                bmplog!("Gdb call\n");
                let s = engine.get_result();
                if s.is_empty() {
                    bmplog!("Cannot read string");
                } else {
                    bmplog!("--> ACK\n");
                    rngdb_send_data_u8(&[CHAR_ACK]);
                    rngdb_output_flush();
                    bmplog!("Exec raw packet\n");
                    if let Err(_e) = bmp::bmp_try(|| commands::exec(s)) {
                        gdb_print!("Stray exception\n");
                        encoder::encoder::reply_e01();
                    } else {
                        bmplog!("Exec done\n");
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
        // update what we consumed
        data_as_slice = &data_as_slice[consumed..];
    }
}
// EOF
