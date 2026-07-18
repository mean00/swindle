// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
#![allow(dead_code)]

pub mod freertos_arm;
pub mod freertos_arm_core;
pub mod freertos_arm_m3;
pub mod freertos_arm_m33;
pub mod freertos_hashtcb;
mod freertos_list;
pub mod freertos_riscv_common;
pub mod freertos_riscv_rv32_std;
pub mod freertos_riscv_rv32_wch;
pub mod freertos_symbols;
pub mod freertos_tcb;
pub mod freertos_trait;
use crate::bmp::bmp_cpuid;
use alloc::boxed::Box;
use core::mem::MaybeUninit;

use freertos_arm::create_arm_switch_handler;
use freertos_riscv_common::create_rv32_switch_handler;
use freertos_symbols::{freertos_clear_symbols, get_symbols};
pub use freertos_tcb::freertos_invalidate_cache;
use freertos_trait::freertos_switch_handler;
pub use freertos_trait::freertos_task_info;

setup_log!(false);
crate::gdb_print_init!();

#[derive(PartialEq, Copy, Clone)]
pub enum LN_MCU_CORE {
    LN_MCU_AUTO,
    LN_MCU_CM0,
    LN_MCU_CM3,
    LN_MCU_CM4,
    LN_MCU_CM33,
    LN_MCU_RV32,
    LN_MCU_NONE,
}

pub struct FreeRTOS_switcher {
    pub switcher: Option<Box<dyn freertos_switch_handler>>,
}

// SAFETY: FreeRTOS_switcher is only used in a single-threaded debugger context.
unsafe impl Send for FreeRTOS_switcher {}
unsafe impl Sync for FreeRTOS_switcher {}

fn get_switcher() -> &'static mut FreeRTOS_switcher {
    static mut SWITCHER: MaybeUninit<FreeRTOS_switcher> = MaybeUninit::uninit();
    static mut SWITCHER_INIT: bool = false;
    unsafe {
        if !SWITCHER_INIT {
            SWITCHER.write(FreeRTOS_switcher { switcher: None });
            SWITCHER_INIT = true;
        }
        SWITCHER.assume_init_mut()
    }
}

fn freertos_switch(cortex: &mut dyn freertos_switch_handler, new_stack: u32) -> u32 {
    bmplog!("Switching to new stack 0x:{:x}\n", new_stack);
    if !cortex.read_cur_registers() {
        bmpwarning!("freertos_switch: read_cur_registers failed\n");
        return 0;
    }
    if !cortex.write_registers_to_stack() {
        bmpwarning!("freertos_switch: write_registers_to_stack failed\n");
        return 0;
    }
    let saved_stack = cortex.get_sp();
    if !cortex.read_registers_from_addr(new_stack) {
        bmpwarning!("freertos_switch: read_registers_from_addr(0x{:x}) failed\n", new_stack);
        return 0;
    }
    if !cortex.write_cur_registers() {
        bmpwarning!("freertos_switch: write_cur_registers failed\n");
        return 0;
    }
    saved_stack
}

pub fn os_attach(cpuid: u32) {
    let all_symbols = get_symbols();
    let core = all_symbols.mcu_handler;
    let switcher: Option<Box<dyn freertos_switch_handler>> = match core {
        LN_MCU_CORE::LN_MCU_CM0 | LN_MCU_CORE::LN_MCU_CM3 | LN_MCU_CORE::LN_MCU_CM4 | LN_MCU_CORE::LN_MCU_CM33 | LN_MCU_CORE::LN_MCU_AUTO => {
             create_arm_switch_handler(core, cpuid)
        },
        LN_MCU_CORE::LN_MCU_RV32 => Some(create_rv32_switch_handler()),
        _ => None,
    };
    get_switcher().switcher = switcher;
}

pub fn os_detach() {
    get_switcher().switcher = None;
    freertos_clear_symbols();
}

pub fn os_can_switch() -> bool {
    get_switcher().switcher.is_some()
}

pub fn freertos_switch_task_action(new_stack: u32) -> u32 {
    let switcher = get_switcher();
    match &mut switcher.switcher {
        Some(e) => freertos_switch(e.as_mut(), new_stack),
        None => panic!("inconsistent"),
    }
}

pub fn enable_freertos(flavor: &str) -> bool {
    let all_symbols = get_symbols();
    let core: LN_MCU_CORE = match flavor.to_uppercase().as_str() {
        "M0" => LN_MCU_CORE::LN_MCU_CM0,
        "M3" => LN_MCU_CORE::LN_MCU_CM3,
        "M4" => LN_MCU_CORE::LN_MCU_CM4,
        "M33" => LN_MCU_CORE::LN_MCU_CM33,
        "RV32" => LN_MCU_CORE::LN_MCU_RV32,
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

    all_symbols.running = all_symbols.valid && core != LN_MCU_CORE::LN_MCU_NONE;

    true
}

pub fn os_info() {
    freertos_hashtcb::dump_hash_info();
}
