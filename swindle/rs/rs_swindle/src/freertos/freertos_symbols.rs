//! FreeRTOS Symbol Resolution and Extraction
//!
//! This module handles resolving critical FreeRTOS internal state from the ELF
//! debug symbols, such as `pxCurrentTCB` and list headers, allowing the debugger
//! to inspect OS state dynamically.

use crate::bmp;
use crate::parsing_util;
use core::mem::MaybeUninit;

use crate::freertos::LN_MCU_CORE;

setup_log!(false);

// Reference the C-side `lnGetFreeRTOSDebug()` to force the linker to pull in
// the `lnFreeRTOSDebug.cpp.obj` from the archive, which contains the
// `freeRTOSDebug` struct needed by the debugger.
unsafe extern "C" {
    fn lnGetFreeRTOSDebug() -> *const u32;
}

use strum::IntoEnumIterator;
use strum_macros::EnumIter;

const NB_FREERTOS_SYMBOLS: usize = 7;
#[derive(PartialEq, EnumIter, Clone, Copy)]
enum freeRtosSymbolIndex {
    pxCurrentTCB = 0,
    xSuspendedTaskList = 1,
    xDelayedTaskList1 = 2,
    xDelayedTaskList2 = 3,
    pxReadyTasksLists = 4,
    xSchedulerRunning = 5,
    freeRTOSDebug = 6,
    invalid = 0xff,
}
pub const FreeRTOSSymbolName: [&str; NB_FREERTOS_SYMBOLS] = [
    "pxCurrentTCB",
    "xSuspendedTaskList",
    "xDelayedTaskList1",
    "xDelayedTaskList2",
    "pxReadyTasksLists",
    "xSchedulerRunning",
    "freeRTOSDebug",
];
/// Runtime FreeRTOS debug offsets read from the target's `freeRTOSDebug` symbol.
/// Mirrors the C `lnFreeRTOSDebug` struct.
#[derive(Clone, Copy, Default)]
pub struct FreeRTOSDebugOffsets {
    pub magic: u32,
    pub list_size: u32,
    pub offset_list_item_next: u32,
    pub offset_list_item_owner: u32,
    pub offset_list_number_of_item: u32,
    pub offset_list_index: u32,
    pub nb_of_priorities: u32,
    pub layout_type: u32,
    pub max_task_name_len: u32,
    pub offset_task_name: u32,
    pub offset_task_num: u32,
}

pub const LAYOUT_ARM_NOFPU: u32 = 0;
pub const LAYOUT_ARM_FPU: u32 = 1;
pub const LAYOUT_RV_STD: u32 = 2;
pub const LAYOUT_RV_STD_FPU: u32 = 3;
pub const LAYOUT_CH32: u32 = 4;
pub const LAYOUT_CH32_FPU: u32 = 5;

/// Global cache of resolved FreeRTOS symbol addresses and OS context configuration.
pub struct FreeRTOSSymbols {
    /// True if FreeRTOS context switching is running
    pub running: bool,
    /// True if all mandatory FreeRTOS symbols have been found
    pub valid: bool,
    /// Cached memory addresses for each `freeRtosSymbolIndex`
    pub addresses: [Option<u32>; NB_FREERTOS_SYMBOLS],
    /// CPU ID detected from hardware
    pub cpuid: u32,
    /// MCU core type layout detected
    pub mcu_handler: LN_MCU_CORE,
    /// Detailed offset mapping extracted from `freeRTOSDebug`
    pub debug_offsets: Option<FreeRTOSDebugOffsets>,
}
// SAFETY: FreeRTOSSymbols is only used in a single-threaded debugger context.
unsafe impl Sync for FreeRTOSSymbols {}

/// Retrieves a mutable reference to the global `FreeRTOSSymbols` cache.
pub fn get_symbols() -> &'static mut FreeRTOSSymbols {
    static mut SYMBOLS: MaybeUninit<FreeRTOSSymbols> = MaybeUninit::uninit();
    static mut SYMBOLS_INIT: bool = false;
    unsafe {
        if !SYMBOLS_INIT {
            SYMBOLS.write(FreeRTOSSymbols {
                running: false,
                valid: false,
                addresses: [None; NB_FREERTOS_SYMBOLS],
                cpuid: 0,
                mcu_handler: LN_MCU_CORE::LN_MCU_NONE,
                debug_offsets: None,
            });
            SYMBOLS_INIT = true;
        }
        SYMBOLS.assume_init_mut()
    }
}

/// Clears all loaded FreeRTOS symbols and marks the context as invalid.
pub fn freertos_clear_symbols() -> bool {
    let all_symbols = get_symbols();
    all_symbols.running = false;
    all_symbols.valid = false;
    all_symbols.addresses = [None; NB_FREERTOS_SYMBOLS];
    all_symbols.debug_offsets = None;
    true
}

/// Helper to convert a string symbol name to the `freeRtosSymbolIndex`.
fn lookup_name(key: &str) -> freeRtosSymbolIndex {
    for i in freeRtosSymbolIndex::iter() {
        if i == freeRtosSymbolIndex::invalid {
            return freeRtosSymbolIndex::invalid;
        }
        let index = i as usize;
        if key == FreeRTOSSymbolName[index] {
            return i;
        }
    }
    freeRtosSymbolIndex::invalid
}

/// Re-evaluates whether all mandatory symbols have been located.
fn update_valid(symbols: &mut FreeRTOSSymbols) {
    let mut missing: bool = false;
    for ref i in symbols.addresses {
        if i.is_none() {
            missing = true;
        }
    }
    symbols.valid = !missing;
}

/// Magic value from C `LN_FREERTOS_MAGIC` to validate the debug struct.
const LN_FREERTOS_MAGIC: u32 = 0x1FEEBAE;

/// Read the `freeRTOSDebug` struct from target memory and populate `debug_offsets`.
/// Called automatically when the `freeRTOSDebug` symbol address is received.
fn freertos_read_debug_offsets() {
    // Force a reference to the C-side `lnGetFreeRTOSDebug()` so the linker
    // pulls in `lnFreeRTOSDebug.cpp.obj` from the archive, making the
    // `freeRTOSDebug` symbol available in every esprit-based binary.
    unsafe { lnGetFreeRTOSDebug(); }

    let all_symbols = get_symbols();
    let addr = match all_symbols.addresses[freeRtosSymbolIndex::freeRTOSDebug as usize] {
        Some(a) => a,
        None => return,
    };
    let mut raw: [u32; 11] = [0; 11];
    if !bmp::bmp_read_mem32(addr, &mut raw) {
        bmpwarning!("Failed to read freeRTOSDebug struct at 0x{:x}\n", addr);
        return;
    }
    if raw[0] != LN_FREERTOS_MAGIC {
        bmpwarning!(
            "freeRTOSDebug magic mismatch: got 0x{:08x}, expected 0x{:08x}\n",
            raw[0],
            LN_FREERTOS_MAGIC
        );
        return;
    }
    all_symbols.debug_offsets = Some(FreeRTOSDebugOffsets {
        magic: raw[0],
        list_size: raw[1],
        offset_list_item_next: raw[2],
        offset_list_item_owner: raw[3],
        offset_list_number_of_item: raw[4],
        offset_list_index: raw[5],
        nb_of_priorities: raw[6],
        layout_type: raw[7],
        max_task_name_len: raw[8],
        offset_task_name: raw[9],
        offset_task_num: raw[10],
    });
    bmplog!("freeRTOSDebug offsets loaded from target\n");
}

/// Processes incoming symbol definitions from GDB and stores them in the cache.
/// 
/// Once the `freeRTOSDebug` symbol is received, this automatically triggers
/// `freertos_read_debug_offsets` to pull the detailed struct layout from memory.
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
    // If this is the freeRTOSDebug symbol, read the struct from target memory
    if index == freeRtosSymbolIndex::freeRTOSDebug {
        freertos_read_debug_offsets();
    }
    update_valid(all_symbols);
    true
}

/// Retrieves the cached address of the `pxCurrentTCB` pointer.
pub fn get_current_tcb_address() -> u32 {
    let all_symbols = get_symbols();
    all_symbols.addresses[freeRtosSymbolIndex::pxCurrentTCB as usize].unwrap()
}

/// Returns whether all mandatory FreeRTOS symbols have been correctly resolved.
pub fn freertos_symbol_valid() -> bool {
    get_symbols().valid
}

/// Returns whether the FreeRTOS OS plugin is currently active and running.
pub fn freertos_running() -> bool {
    get_symbols().running
}
