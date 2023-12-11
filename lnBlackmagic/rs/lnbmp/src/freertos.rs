// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html

use alloc::vec::Vec;
pub mod freertos_trait;
mod freertos_list;
pub mod freertos_symbols;
pub mod freertos_hashtcb;
pub mod freertos_tcb;
pub mod freertos_arm_core;
pub mod freertos_arm;
pub mod freertos_arm_m0;
pub mod freertos_arm_m3;

use crate::bmp::{bmp_read_mem,bmp_read_mem32};
use freertos_trait::{freertos_task_info};
use freertos_symbols::{get_symbols};
use freertos_arm_core::freertos_cortexm_core;
use freertos_arm::{freertos_attach_arm, freertos_detach_arm, freertos_can_switch_arm, freertos_switch_task_action_arm};


crate::setup_log!(false);
crate::gdb_print_init!();
use crate::{bmplog, bmpwarning,gdb_print};

#[derive(PartialEq,Copy,Clone)]
pub enum LN_MCU_CORE {
        LN_MCU_AUTO,
        LN_MCU_CM0,
        LN_MCU_CM3,
        LN_MCU_CM4,
        LN_MCU_CM33,
        LN_MCU_NONE, 
}

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
pub fn enable_freertos(flavor : &str) -> bool
{
    let all_symbols = get_symbols();
    let core : LN_MCU_CORE = match flavor.to_uppercase().as_str() {
        "M0"  =>  LN_MCU_CORE::LN_MCU_CM0,
        "M3"  =>  LN_MCU_CORE::LN_MCU_CM3,
        "M4"  =>  LN_MCU_CORE::LN_MCU_CM4,
        "M33" =>  LN_MCU_CORE::LN_MCU_CM33,
        "" | "AUTO"=>  LN_MCU_CORE::LN_MCU_AUTO,
        "NONE" | "OFF" => LN_MCU_CORE::LN_MCU_NONE,        
        _     => { gdb_print!("Unkown core: None, auto, cm0, cm3, cm4 or cm33\n"); return false;},
    };    
    all_symbols.mcu_handler = core;
    all_symbols.valid = all_symbols.loaded && core!=LN_MCU_CORE::LN_MCU_NONE;    
    all_symbols.valid
}
//

 