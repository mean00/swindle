use alloc::vec::Vec;
pub mod freertos_trait;
mod freertos_list;
pub mod freertos_symbols;
pub mod freertos_tcb;
pub mod freertos_arm_m0;

use crate::bmp::{bmp_read_mem,bmp_read_mem32};
use freertos_trait::{freertos_task_info};
use freertos_symbols::{get_symbols};

crate::setup_log!(false);
crate::gdb_print_init!();
use crate::{bmplog, bmpwarning};


/**
 * 
 */
pub fn enable_freertos() -> bool
{
    let all_symbols = get_symbols();
    all_symbols.valid = all_symbols.loaded;
    return all_symbols.valid;
}


pub fn fos_taist() 
{
    // the common part of TCB is 5 registers
    // pxTopOfStack
    // *State
    // *EventList
    // priority
    // stack
            
    let r=crate::freertos::freertos_tcb::freertos_collect_information(); 
    for i in r
    {
        i.print_tcb();
    }    
}

 //