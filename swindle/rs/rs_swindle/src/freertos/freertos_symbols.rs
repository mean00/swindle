//use crate::bmp::{bmp_read_mem, bmp_read_mem32};
use crate::parsing_util;
use core::ptr::addr_of_mut;

//use crate::freertos::freertos_trait::{freertos_task_info, freertos_task_state};
use crate::freertos::LN_MCU_CORE;

crate::setup_log!(true);
use crate::{bmplog, bmpwarning};

use strum::IntoEnumIterator;
use strum_macros::EnumIter;

const NB_FREERTOS_SYMBOLS: usize = 6;
#[derive(PartialEq, EnumIter, Clone)]
enum freeRtosSymbolIndex {
    pxCurrentTCB = 0,
    xSuspendedTaskList = 1,
    xDelayedTaskList1 = 2,
    xDelayedTaskList2 = 3,
    pxReadyTasksLists = 4,
    xSchedulerRunning = 5,
    invalid = 0xff,
}
pub const FreeRTOSSymbolName: [&str; NB_FREERTOS_SYMBOLS] = [
    "pxCurrentTCB",
    "xSuspendedTaskList",
    "xDelayedTaskList1",
    "xDelayedTaskList2",
    "pxReadyTasksLists",
    "xSchedulerRunning",
];
pub struct FreeRTOSSymbols {
    pub valid: bool,
    pub addresses: [Option<u32>; NB_FREERTOS_SYMBOLS],
    pub cpuid: u32,
    pub mcu_handler: LN_MCU_CORE,
}

static mut freeRtosSymbols_internal: FreeRTOSSymbols = FreeRTOSSymbols {
    valid: false,
    addresses: [None; NB_FREERTOS_SYMBOLS],
    cpuid: 0,
    mcu_handler: LN_MCU_CORE::LN_MCU_NONE,
};
/*
 *
 */
pub fn get_symbols() -> &'static mut FreeRTOSSymbols {
    unsafe { &mut *addr_of_mut!(freeRtosSymbols_internal) }
}
/*
 * \brief : clear the loaded symbols
 */

pub fn freertos_clear_symbols() -> bool {
    let all_symbols = get_symbols();
    all_symbols.valid = false;
    all_symbols.addresses = [None; NB_FREERTOS_SYMBOLS];
    true
}
/*
 *
 *
 */
fn lookup_name(key: &str) -> freeRtosSymbolIndex {
    for i in freeRtosSymbolIndex::iter() {
        if i == freeRtosSymbolIndex::invalid {
            return freeRtosSymbolIndex::invalid;
        }
        let index = i.clone() as usize;
        if key == FreeRTOSSymbolName[index] {
            return i;
        }
    }
    freeRtosSymbolIndex::invalid
}
/*
 *
 *
 */
fn update_valid(symbols: &mut FreeRTOSSymbols) {
    let mut missing: bool = false;
    for ref i in symbols.addresses {
        //if let Some(x) = i {
        if i.is_none() {
            missing = true;
        }
    }
    symbols.valid = !missing;
}
/*
 *
 *
 */
#[unsafe(no_mangle)]
pub fn freertos_processing(key: &str, value_str: &str) -> bool {
    let all_symbols = get_symbols();
    let value = parsing_util::ascii_hex_to_u32(value_str);
    bmplog!(
        "\tprocessing :key {} value {}=>0x{:x}\n",
        key,
        value_str,
        value
    );
    // lookup key
    let index = lookup_name(key);
    if index == freeRtosSymbolIndex::invalid {
        bmpwarning!("That key does not exist\n");
        return false;
    }
    all_symbols.addresses[index as usize] = Some(value);
    update_valid(all_symbols);
    true
}
/*
 *
 */
pub fn get_current_tcb_address() -> u32 {
    let all_symbols = get_symbols();
    all_symbols.addresses[freeRtosSymbolIndex::pxCurrentTCB as usize].unwrap()
}
/*
 *
 *
 */
pub fn freertos_symbol_valid() -> bool {
    get_symbols().valid
}
//
// EOF
