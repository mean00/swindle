// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec::Vec;

use crate::encoder::encoder;

use crate::parsing_util;

crate::setup_log!(false);
use crate::bmplog;

use crate::freertos::freertos_symbols::freertos_running;
use crate::freertos::freertos_tcb::get_threads;
use crate::freertos::freertos_tcb::{freertos_is_thread_present, freertos_switch_task};
use crate::freertos::freertos_tcb::{get_current_thread_id, get_tcb_info_from_id};

crate::gdb_print_init!();
use crate::gdb_print;
//
// ‘m thread-id’
// A single thread ID
// ‘m thread-id,thread-id…’
// a comma-separated list of thread IDs
//
// ‘l’
// (lower case letter ‘L’) denotes end of list.
//
pub fn _qfThreadInfo(_command: &str, _args: &[&str]) -> bool {
    let list = get_threads();
    if list.is_empty() {
        encoder::simple_send("m1");
    } else {
        let mut e = encoder::new();
        e.begin();
        e.add("m");
        let mut not_first: bool = false;
        for i in list {
            if not_first {
                e.add(",");
            }
            e.add_u32_no_padding(i);
            not_first = true;
        }
        e.end();
    }
    true
}
/*
 * get a human readable attributes  "qThreadExtraInfo,id"
 */
pub fn _qThreadExtraInfo(command: &str, _args: &[&str]) -> bool {
    let args: Vec<&str> = command.split(',').collect();
    if args.len() != 2 {
        encoder::reply_e01();
    }
    let thread_id = parsing_util::ascii_string_hex_to_u32(args[1]);

    let info = get_tcb_info_from_id(thread_id);
    if let Some(x) = info {
        let mut e = encoder::new();
        e.begin();
        let name_as_string = unsafe { core::str::from_utf8_unchecked(&x.name) };
        e.hex_and_add("Thread:");
        e.hex_and_add(name_as_string);
        e.hex_and_add(" state:");
        e.hex_and_add(x.state.as_str());
        e.end();
    } else {
        bmplog!("Thread not found (extra)\n");
        encoder::reply_e01();
    }

    true
}
/*
 *  switch thread
 */
pub fn _Hg(_command: &str, args: &[&str]) -> bool {
    let thread_id: u32 = parsing_util::ascii_string_hex_to_u32(args[0]);
    bmplog!("Thread switch to 0x{:x} \n", thread_id);
    if !freertos_running() {
        bmplog!("invalid  thread 0x{:x} \n", thread_id);
        encoder::reply_ok();
        return true;
    }
    if freertos_switch_task(thread_id) {
        bmplog!(" switch ok   thread 0x{:x} \n", thread_id);
        encoder::reply_ok();
    } else {
        bmplog!("failed to switch   thread 0x{:x} \n", thread_id);
        encoder::reply_e01();
    }
    true
}
/*
 *  is thread alive ?
 */
pub fn _T(_command: &str, args: &[&str]) -> bool {
    let thread_id: u32 = parsing_util::ascii_string_hex_to_u32(args[0]);

    let ok = match freertos_running() {
        true => freertos_is_thread_present(thread_id),
        false => thread_id == 1,
    };

    if ok {
        encoder::reply_ok();
    } else {
        encoder::reply_e01();
    }
    true
}
/*
 * get a human readable attributes  "qThreadExtraInfo,id"
 */
pub fn _qP(_command: &str, _args: &[&str]) -> bool {
    encoder::reply_e01();
    true
}

//
//
//
pub fn _qsThreadInfo(_command: &str, _args: &[&str]) -> bool {
    //let list = crate::freertos::freertos_tcb::get_threads();
    // if list.is_empty()  {
    //     encoder::simple_send("l");
    // }
    // else  {
    encoder::simple_send("l");
    // }
    true
}

/*
 * ‘qC’ Return the current thread ID.
 */

pub fn _qC(_command: &str, _args: &[&str]) -> bool {
    let mut current_thread_id = 1;
    if !freertos_running() {
        current_thread_id = 1;
    } else if let Some(x) = get_current_thread_id() {
        current_thread_id = x;
    }

    let mut e = encoder::new();
    e.begin();
    e.add("QC");
    e.add_u32(current_thread_id);
    e.end();
    true
}

/*
 *
 */
pub fn _qSymbol(_command: &str, args: &[&str]) -> bool {
    if args.len() != 2 {
        gdb_print!("Malformed reply to qsymbol\n");
    } else {
        crate::commands::symbols::q_symbols(args);
    }
    encoder::reply_ok();
    true
}

// EOF
