// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec;
use alloc::vec::Vec;

use super::{exec_one, CommandTree};
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

use crate::freertos::freertos_tcb::{get_current_thread_id,freertos_collect_information, get_tcb_info_from_id};

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
// select thread
pub fn _Hg(command: &str, _args: &[&str]) -> bool {

    let thread_id: u32 = parsing_util::ascii_string_to_u32( &command[2..]);
    let info = get_tcb_info_from_id(thread_id);
    if info.is_none() {
        encoder::reply_e01();
        return true;
    }
    // switch to that thread..    
    bmplog!("Hey there\n");
    encoder::reply_e01();
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
/**
 * \fn return a copy of pxCurrentTCB
 */
pub fn get_pxCurrentTCB() -> Option <u32>
{
  None
}



// EOF
