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
    if list.is_empty()
    {
        encoder::simple_send("m1");
    }else
    {
        let mut e = encoder::new();
        e.begin();
        e.add("m ");
        for i in list
        {
            e.add_u32(i);
            e.add(",");
        }
        e.end();
    }
    true
}
//
//
pub fn _qsThreadInfo(_command: &str, _args: &[&str]) -> bool {
    let list = crate::freertos::freertos_tcb::get_threads();
    if list.is_empty()
    {
        encoder::simple_send("l");
    }else
    {
        encoder::simple_send("l");
    }
    true
}
// get current thread I
pub fn _qC(_command: &str, _args: &[&str]) -> bool {
    encoder::simple_send("QC1");
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
