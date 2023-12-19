// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html

use alloc::vec::Vec;
pub mod freertos_arm;
pub mod freertos_arm_core;
pub mod freertos_arm_m0;
pub mod freertos_arm_m3;
pub mod freertos_arm_m33;
pub mod freertos_hashtcb;
mod freertos_list;
pub mod freertos_symbols;
pub mod freertos_tcb;
pub mod freertos_trait;
use crate::bmp::bmp_cpuid;
use crate::bmp::{bmp_read_mem, bmp_read_mem32};
use freertos_arm::{
    freertos_attach_arm, freertos_can_switch_arm, freertos_detach_arm,
    freertos_switch_task_action_arm,
};
use freertos_arm_core::freertos_cortexm_core;
use freertos_symbols::get_symbols;
use freertos_trait::freertos_task_info;

crate::setup_log!(false);
crate::gdb_print_init!();
use crate::{bmplog, bmpwarning, gdb_print};

#[derive(PartialEq, Copy, Clone)]
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
pub fn os_attach(cpuid: u32) {
    freertos_attach_arm(cpuid);
}
/**
 *
 */
pub fn os_detach() {
    freertos_detach_arm();
}
/**
 *
 */
pub fn os_can_switch() -> bool {
    freertos_can_switch_arm()
}

/**
 *
 */
pub fn freertos_switch_task_action(new_stack: u32) -> u32 {
    freertos_switch_task_action_arm(new_stack)
}
/**
 *
 */
pub fn enable_freertos(flavor: &str) -> bool {
    let all_symbols = get_symbols();
    let core: LN_MCU_CORE = match flavor.to_uppercase().as_str() {
        "M0" => LN_MCU_CORE::LN_MCU_CM0,
        "M3" => LN_MCU_CORE::LN_MCU_CM3,
        "M4" => LN_MCU_CORE::LN_MCU_CM4,
        "M33" => LN_MCU_CORE::LN_MCU_CM33,
        "" | "AUTO" => LN_MCU_CORE::LN_MCU_AUTO,
        "NONE" | "OFF" => {
            os_detach();
            LN_MCU_CORE::LN_MCU_NONE
        }
        _ => {
            return false;
        }
    };
    all_symbols.mcu_handler = core;
    os_attach(bmp_cpuid());

    all_symbols.valid = all_symbols.loaded && core != LN_MCU_CORE::LN_MCU_NONE;

    true
}
/**
 *
 */

pub fn os_info() {
    freertos_hashtcb::dump_hash_info();
}

//
