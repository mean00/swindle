
pub mod freertos_trait;
pub mod freertos_symbols;
use crate::bmp::{bmp_read_mem,bmp_read_mem32};
use freertos_trait::{freertos_task_info, freertos_handler};
crate::setup_log!(false);
crate::gdb_print_init!();
use crate::{bmplog, bmpwarning};

pub fn spawn_freertos_handler() -> Option<&'static dyn freertos_handler>
{
    None
}

pub fn fos_taist() 
{
    read_fos(0);
    /*
    let tcb = crate::commands::q::get_pxCurrentTCB();
    if let Some(x) = tcb
    {
        read_fos(x);
    }else   {
        bmplog!("pxCurrentTcb not available \n");
    }
    */
}
/**
 * 
 */
pub fn read_fos(address : u32) -> Option<freertos_task_info>
{

    // the common part of TCB is 5 registers
    // pxTopOfStack
    // *State
    // *EventList
    // priority
    // stack
    unsafe {
    crate::freertos::freertos_symbols::freertos_collect_information();    
    }
    return None;
}
 //