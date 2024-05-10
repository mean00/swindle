/*
    Here we dispatch to get the correct task switching code depending on the
    ARM core / FreeRTOS implementation

*/
crate::setup_log!(false);
use crate::{bmplog, bmpwarning, gdb_print};

crate::gdb_print_init!();
use core::ops::DerefMut;

use crate::bmp::{bmp_read_mem, bmp_read_mem32, bmp_write_mem32};
use crate::freertos::freertos_arm_core::freertos_cortexm_core;
use crate::freertos::freertos_arm_m0::freertos_switch_handler_m0;
use crate::freertos::freertos_arm_m3::freertos_switch_handler_m3;
use crate::freertos::freertos_arm_m33::freertos_switch_handler_m33;
use crate::freertos::freertos_riscv_rv32::freertos_switch_handler_rv32;
use crate::freertos::freertos_symbols::get_symbols;
use crate::freertos::freertos_trait::{
    freertos_switch_handler, freertos_task_info, freertos_task_state,
};
use crate::freertos::LN_MCU_CORE;
use alloc::boxed::Box;
use core::ptr::null;

const ARM_PARTNO_MASK: u32 = 0xfff0;

const ARM_CM0: u32 = 0xc200;
const ARM_CM0P: u32 = 0xc600;
const ARM_CM3: u32 = 0xc230;
const ARM_CM4: u32 = 0xc240;
const ARM_CM7: u32 = 0xc270;
const ARM_CM23: u32 = 0xd200;
const ARM_CM33: u32 = 0xd210;

pub struct FreeRTOS_switcher {
    pub switcher: Option<Box<dyn freertos_switch_handler>>,
}

static mut FreeRTOS_switcher_internal: FreeRTOS_switcher = FreeRTOS_switcher { switcher: None };
/*
 *
 */

fn freertos_switch(cortex: &mut dyn freertos_switch_handler, new_stack: u32) -> u32 {
    // load current regs into cortex
    cortex.read_current_registers();
    // save on to tcb
    cortex.write_registers_to_stack();
    // Updated stack of old thread
    let saved_stack = cortex.get_sp();
    // ok , old thread has been saved, now restore new thread
    // restore registers
    cortex.read_registers_from_addr(new_stack);
    // update actual reg from copy in cortex
    cortex.write_current_registers();
    saved_stack
}
/*
 * switch to new TCB whose stack is new_stack
 * returns the old TCB new stack value
 */
pub fn freertos_switch_task_action_arm(new_stack: u32) -> u32 {
    unsafe {
        return match &mut FreeRTOS_switcher_internal.switcher {
            Some(e) => freertos_switch(e.as_mut(), new_stack),
            None => panic!("inconsistent"),
        };
    }
}

/*
 *
 */
const mul: u32 = 0;
pub fn freertos_attach_arm(cpu: u32) -> bool {
    // Lookup the
    let all_symbols = get_symbols();
    let mut core = all_symbols.mcu_handler;
    if core == LN_MCU_CORE::LN_MCU_AUTO {
        core = match (mul * ARM_CM33 + (1 - mul) * cpu) & ARM_PARTNO_MASK {
            ARM_CM0 | ARM_CM0P => LN_MCU_CORE::LN_MCU_CM0,
            ARM_CM3 => LN_MCU_CORE::LN_MCU_CM3,
            ARM_CM4 => LN_MCU_CORE::LN_MCU_CM4,
            ARM_CM33 => LN_MCU_CORE::LN_MCU_CM33,
            _ => {
                return false;
            }
        };
    }
    unsafe {
        let switcher: Box<dyn freertos_switch_handler> = match core {
            LN_MCU_CORE::LN_MCU_CM0 => Box::new(freertos_switch_handler_m0::new()),
            LN_MCU_CORE::LN_MCU_CM3 => Box::new(freertos_switch_handler_m3::new()),
            LN_MCU_CORE::LN_MCU_CM4 => Box::new(freertos_switch_handler_m3::new()),
            LN_MCU_CORE::LN_MCU_CM33 => Box::new(freertos_switch_handler_m33::new()),
            LN_MCU_CORE::LN_MCU_RV32 => Box::new(freertos_switch_handler_rv32::new()),
            _ => {
                return false;
            }
        };
        FreeRTOS_switcher_internal.switcher = Some(switcher);
    } // unsafe
    true
}

pub fn freertos_can_switch_arm() -> bool {
    unsafe { FreeRTOS_switcher_internal.switcher.is_some() }
}

/*
 *
 */
pub fn freertos_detach_arm() {
    unsafe {
        FreeRTOS_switcher_internal.switcher = None;
    }
}
