// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html

use alloc::vec::Vec;
pub mod freertos_trait;
mod freertos_list;
pub mod freertos_symbols;
pub mod freertos_hashtcb;
pub mod freertos_tcb;
pub mod freertos_arm;
pub mod freertos_arm_m0;
pub mod freertos_arm_m3;

use crate::bmp::{bmp_read_mem,bmp_read_mem32};
use freertos_trait::{freertos_task_info};
use freertos_symbols::{get_symbols};
use freertos_arm::{freertos_attach_arm, freertos_detach_arm, freertos_can_switch_arm, freertos_switch_task_action_arm};

crate::setup_log!(false);
crate::gdb_print_init!();
use crate::{bmplog, bmpwarning};

/**
 * 
 */
pub fn os_attach( cpuid : u32) {
    freertos_attach_arm(cpuid);
}
/**
 * 
 */
pub fn os_detach( ) {
    freertos_detach_arm();
}
/**
 * 
 */
pub fn os_can_switch() ->bool {
    freertos_can_switch_arm()
}
/**
 * 
 */
pub fn freertos_switch_task_action( new_stack : u32) -> u32{
   freertos_switch_task_action_arm( new_stack )
}
/**
 * 
 */
pub fn enable_freertos() -> bool
{
    let all_symbols = get_symbols();
    all_symbols.valid = all_symbols.loaded;
    all_symbols.valid
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
