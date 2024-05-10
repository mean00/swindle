use crate::bmp::{bmp_read_mem, bmp_read_mem32};
use crate::encoder::encoder;
use crate::parsing_util;
use alloc::vec::Vec;
use core::ptr::addr_of_mut;

use crate::freertos::freertos_trait::{freertos_task_info, freertos_task_state};
use crate::freertos::LN_MCU_CORE;

crate::setup_log!(false);
use crate::{bmplog, bmpwarning};

const NB_FREERTOS_SYMBOLS: usize = 6;

enum freeRtosSymbolIndex {
    pxCurrentTCB = 0,
    xSuspendedTaskList = 1,
    xDelayedTaskList1 = 2,
    xDelayedTaskList2 = 3,
    pxReadyTasksLists = 4,
    xSchedulerRunning = 5,
}
const FreeRTOSSymbolName: [&str; NB_FREERTOS_SYMBOLS] = [
    "pxCurrentTCB",
    "xSuspendedTaskList",
    "xDelayedTaskList1",
    "xDelayedTaskList2",
    "pxReadyTasksLists",
    "xSchedulerRunning",
];
pub struct FreeRTOSSymbols {
    pub valid: bool,
    pub loaded: bool,
    pub index: usize,
    pub symbols: [u32; NB_FREERTOS_SYMBOLS],
    pub cpuid: u32,
    pub mcu_handler: LN_MCU_CORE,
}

static mut freeRtosSymbols_internal: FreeRTOSSymbols = FreeRTOSSymbols {
    valid: false,
    loaded: false,
    index: 0,
    symbols: [0; NB_FREERTOS_SYMBOLS],
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
 * \brief : ask gdb for pxCurrentTCB address
 */

fn ask_for_next_symbol() -> bool {
    let mut e = encoder::new();
    e.begin();
    e.add("qSymbol:");
    let symbol = get_symbols();
    e.hex_and_add(FreeRTOSSymbolName[symbol.index]);
    e.end();
    true
}
/*
 *
 */
pub fn q_freertos_symbols(args: &[&str]) -> bool {
    let all_symbols = get_symbols();
    if all_symbols.valid {
        encoder::reply_ok();
        return true;
    }
    if args.len() != 2 {
        bmpwarning!("Incorrect reply size {}", args.len());
        encoder::reply_e01();
        return true;
    }
    // is it an empty one ?, if so ask for pxCurrentTcb
    if args[0].is_empty() && args[1].is_empty() {
        all_symbols.index = 0;
        all_symbols.valid = false;
        return ask_for_next_symbol();
    }
    // Ok what is the symbol ?
    let mut symbol: [u8; 32] = [0; 32];
    let clear_text = parsing_util::ascii_hex_string_to_str(args[1], &mut symbol);
    if let Ok(text) = clear_text {
        if text.eq(FreeRTOSSymbolName[all_symbols.index]) {
            if args[0].is_empty() {
                bmpwarning!("cannot find symbol {}\n", text);
                encoder::reply_ok();
                return true;
            } else {
                // next
                let address: u32 = parsing_util::ascii_string_to_u32(args[0]);
                bmplog!(
                    "Found symbol {} : 0x{:x}\n",
                    FreeRTOSSymbolName[all_symbols.index],
                    address
                );
                all_symbols.symbols[all_symbols.index] = address;
                all_symbols.index += 1;
                if all_symbols.index == NB_FREERTOS_SYMBOLS {
                    bmplog!("Got all symbols\n");
                    all_symbols.loaded = true;
                    encoder::reply_ok();
                    return true;
                }
                return ask_for_next_symbol();
            }
        } else {
            bmpwarning!(
                "Inconsistent reply for index {}, reply is {}\n",
                all_symbols.index,
                text
            );
        }
    }
    bmpwarning!("Invalid qsymbol reply\n");
    false
}

/*
 *
 */
pub fn get_current_tcb_address() -> u32 {
    let all_symbols = get_symbols();
    let px_adr: u32 = all_symbols.symbols[freeRtosSymbolIndex::pxCurrentTCB as usize];
    px_adr
}
/*
 *
 */
pub fn freertos_symbol_valid() -> bool {
    let all_symbols = get_symbols();
    all_symbols.valid
}
// EOF
