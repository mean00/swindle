// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec;
use alloc::vec::Vec;

use super::{exec_one, CommandTree};
use crate::bmp::{bmp_write_mem32,bmp_read_mem32};
use crate::encoder::encoder;
use crate::packet_symbols::INPUT_BUFFER_SIZE;

use super::mon::_swdp_scan;
use crate::bmp::bmp_attached;
use crate::bmp::bmp_crc32;
use crate::bmp::bmp_get_mapping;
use crate::bmp::mapping::{Flash, Ram};
use crate::bmp::MemoryBlock;
use crate::commands::mon::_qRcmd;
use crate::commands::CallbackType;
use crate::util::xmin;
use crate::parsing_util;
use numtoa::NumToA;

crate::setup_log!(true);
use crate::{bmplog, bmpwarning};

use crate::freertos::freertos_trait::{freertos_switch_handler,freertos_task_info};
use crate::freertos::freertos_tcb::{get_current_thread_id,freertos_collect_information, get_tcb_info_from_id};
use crate::freertos::freertos_arm_m0::freertos_switch_handler_m0;
use crate::freertos::freertos_tcb::{set_pxCurrentTCB, get_pxCurrentTCB};

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

    let list = crate::freertos::freertos_tcb::get_threads();
    if list.is_empty()    {
        encoder::simple_send("m1");
    }else    {
        let mut e = encoder::new();
        e.begin();
        e.add("m");
        let mut not_first: bool = false ; 
        for i in list {
            if not_first
            {
                e.add(",");
            }
            e.add_u32_no_padding(i);
            not_first = true;
         
        }
        e.end();
    }
    true
}
/**
 * get a human readable attributes  "qThreadExtraInfo,id"
 */
pub fn _qThreadExtraInfo(command: &str, _args: &[&str]) -> bool 
{   
    let args: Vec<&str> = command.split(',').collect();
    if args.len()!=2 {
            encoder::reply_e01();
    }
    let thread_id = parsing_util::ascii_string_to_u32(args[1]);

    let info = get_tcb_info_from_id(thread_id);
    if let Some(x) = info     {
        let mut e = encoder::new();
        e.begin();
        let name_as_string = unsafe { core::str::from_utf8_unchecked(&x.name) };
        e.hex_and_add("Thread:");
        e.hex_and_add(name_as_string);
        e.hex_and_add(" state:");
        e.hex_and_add(x.state.as_str());
        e.end();
    }
    else     {
        encoder::reply_e01();
    }

    true
}
/**
 *  switch thread
 */
pub fn _Hg(command: &str, _args: &[&str]) -> bool {

    let thread_id: u32 = parsing_util::ascii_string_to_u32( &command[2..]);

    let new_info = get_tcb_info_from_id(thread_id);
    let new_tcb : freertos_task_info;
    // read new tcb 
    match new_info {
        None =>  {  encoder::reply_e01();   return true;  },
        Some(x) => new_tcb =x,
    };
    // read old tcb, exit if it is actually the same as the new one
    let old_current_tcb_adr : u32;
    match get_pxCurrentTCB()
    {
        None => {  encoder::reply_e01();   return true;  },
        Some(x) => { if x==new_tcb.tcb_addr  {  encoder::reply_ok();return true;}  old_current_tcb_adr = x; },
    }

    let cortex : &mut dyn freertos_switch_handler;

    let mut m0 =  freertos_switch_handler_m0::new();
    cortex = &mut m0 ;

    // read current reg
    cortex.read_current_registers();
    // save on to tcb
    cortex.write_registers_to_stack();
    let saved_stack = cortex.get_sp();
    // write new top of stack
    let mut item : [u32;1] = [old_current_tcb_adr];
    bmp_write_mem32(old_current_tcb_adr, &item );

    // ok , old thread has been saved, now restore new thread
    // top of stack is still 1st item in tcb
    bmp_read_mem32(new_tcb.top_of_stack, &mut item );    
    // restore registers
    cortex.read_registers_from_addr(item[0]);
    // update actual reg from copy in cortex
    cortex.write_current_registers();
    // restore register
    // switch to that thread..  
    set_pxCurrentTCB(new_tcb.tcb_addr);    
    encoder::reply_ok();
    true
}

/**
 * get a human readable attributes  "qThreadExtraInfo,id"
 */
pub fn _qP(_command: &str, _args: &[&str]) -> bool 
{   
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
/**
 * ‘qC’ Return the current thread ID.
 */

pub fn _qC(_command: &str, _args: &[&str]) -> bool {
    let mut current_thread_id =1;
    if let Some(x) = get_current_thread_id() {
        current_thread_id = x;
    }
    let mut e = encoder::new();
    e.begin();
    e.add("QC");    
    e.add_u32(current_thread_id);    
    e.end();    
    true
}


/**
 * 
 */
pub fn _qSymbol(_command: &str, args: &[&str]) -> bool {
    crate::freertos::freertos_symbols::q_freertos_symbols(args)
}



// EOF
