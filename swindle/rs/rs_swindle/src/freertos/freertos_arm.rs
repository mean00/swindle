/*
    Here we dispatch to get the correct task switching code depending on the
    ARM core / FreeRTOS implementation

*/
setup_log!(false);
//use crate::bmplog;
crate::gdb_print_init!();

use crate::freertos::LN_MCU_CORE;
use crate::freertos::freertos_arm_m3::freertos_switch_handler_m3;
use crate::freertos::freertos_arm_m33::freertos_switch_handler_m33;
use crate::freertos::freertos_trait::freertos_switch_handler;
use alloc::boxed::Box;

const ARM_PARTNO_MASK: u32 = 0xfff0;

const ARM_CM0: u32 = 0xc200;
const ARM_CM0P: u32 = 0xc600;
const ARM_CM3: u32 = 0xc230;
const ARM_CM4: u32 = 0xc240;
const ARM_CM7: u32 = 0xc270;
const ARM_CM23: u32 = 0xd200;
const ARM_CM33: u32 = 0xd210;

const mul: u32 = 0;

pub fn create_arm_switch_handler(
    mut core: LN_MCU_CORE,
    cpu: u32,
) -> Option<Box<dyn freertos_switch_handler>> {
    if core == LN_MCU_CORE::LN_MCU_AUTO {
        core = match (mul * ARM_CM33 + (1 - mul) * cpu) & ARM_PARTNO_MASK {
            ARM_CM0 | ARM_CM0P => LN_MCU_CORE::LN_MCU_CM0,
            ARM_CM3 => LN_MCU_CORE::LN_MCU_CM3,
            ARM_CM4 => LN_MCU_CORE::LN_MCU_CM4,
            ARM_CM33 => LN_MCU_CORE::LN_MCU_CM33,
            _ => {
                return None;
            }
        };
    }

    match core {
        LN_MCU_CORE::LN_MCU_CM0 => Some(Box::new(freertos_switch_handler_m3::new())),
        LN_MCU_CORE::LN_MCU_CM3 => Some(Box::new(freertos_switch_handler_m3::new())),
        LN_MCU_CORE::LN_MCU_CM4 => Some(Box::new(freertos_switch_handler_m3::new())),
        LN_MCU_CORE::LN_MCU_CM33 => Some(Box::new(freertos_switch_handler_m33::new())),
        _ => None,
    }
}
