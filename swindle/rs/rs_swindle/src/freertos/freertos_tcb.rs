use crate::bmp::{bmp_read_mem, bmp_read_mem32, bmp_write_mem32};
use alloc::vec::Vec;
use core::mem::MaybeUninit;

use crate::freertos::freertos_hashtcb::get_hashtcb;
use crate::freertos::freertos_list::freertos_crawl_list;

use crate::freertos::freertos_symbols::{
    get_current_tcb_address, get_symbols, FreeRTOSDebugOffsets, FreeRTOSSymbolName,
};
use crate::freertos::freertos_trait::freertos_task_state;

use crate::freertos::{freertos_switch_task_action, freertos_task_info, os_can_switch};

setup_log!(false);
//use crate::{bmplog, bmpwarning};

/// Default hardcoded offsets used as fallback when the target doesn't export `freeRTOSDebug`.
const FALLBACK_OFFSETS: FreeRTOSDebugOffsets = FreeRTOSDebugOffsets {
    magic: 0,
    list_size: 0,
    offset_list_item_next: 4,
    offset_list_item_owner: 12,
    offset_list_number_of_item: 0,
    offset_list_index: 4,
    nb_of_priorities: 16,
    layout_type: 0,
    max_task_name_len: 16,
    offset_task_name: 52,
    offset_task_num: 68,
};

/// Get the effective debug offsets: from target if available, otherwise fallback.
fn get_debug_offsets() -> FreeRTOSDebugOffsets {
    get_symbols()
        .debug_offsets
        .unwrap_or(FALLBACK_OFFSETS)
}

const map_state: [freertos_task_state; 5] = [
    freertos_task_state::running,
    freertos_task_state::suspended,
    freertos_task_state::blocked,
    freertos_task_state::blocked,
    freertos_task_state::ready,
];

//--
// Cache for freertos_collect_information() results.
// Avoids re-scanning target memory on every GDB thread query.
// Invalidated when the target resumes execution.
// SAFETY: single-threaded debugger context.
struct TcbCache {
    data: Vec<freertos_task_info>,
    dirty: bool,
}

unsafe impl Sync for TcbCache {}

fn get_cache() -> &'static mut TcbCache {
    static mut CACHE: MaybeUninit<TcbCache> = MaybeUninit::uninit();
    static mut CACHE_INIT: bool = false;
    unsafe {
        if !CACHE_INIT {
            CACHE.write(TcbCache {
                data: Vec::new(),
                dirty: true,
            });
            CACHE_INIT = true;
        }
        CACHE.assume_init_mut()
    }
}

/// Invalidate the TCB cache. Call this when the target resumes execution.
pub fn freertos_invalidate_cache() {
    let cache = get_cache();
    cache.dirty = true;
    cache.data.clear();
}

//--
/*
 *
 */

fn read_tcb(tcb: u32, state: freertos_task_state) -> Option<freertos_task_info> {
    let off = get_debug_offsets();
    let mut data: [u32; 16] = [0; 16];
    if !bmp_read_mem32(tcb, &mut data[0..1]) {
        bmpwarning!("cannot read TCB\n");
        return None;
    }
    let topOfStack: u32 = data[0];
    // Read priority and stack pointer from TCB.
    // uxPriority and pxStack are always the two 4-byte fields immediately
    // before pcTaskName, regardless of MPU wrappers or other config.
    // Derive their offsets from the already-known offset_task_name.
    let priority_off = off.offset_task_name - 8; // uxPriority (pxStack follows at +4)
    if !bmp_read_mem32(tcb + priority_off, &mut data[0..2]) {
        bmpwarning!("cannot read TCB\n");
        return None;
    }
    let priority = data[0];
    let stack = data[1];

    // Read task name from dynamic offset
    let name_len = off.max_task_name_len as usize;
    let mut name: Vec<u8> = vec![b' '; name_len];
    if !bmp_read_mem(tcb + off.offset_task_name, &mut name) {
        bmpwarning!("cannot read name\n");
        return None;
    }
    // Trim trailing whitespace/null for the fixed-size array
    let trimmed_len = name.iter().rposition(|&c| c != b' ' && c != 0).map(|i| i + 1).unwrap_or(0);
    let mut name_arr: [u8; 8] = [b' '; 8];
    let copy_len = core::cmp::min(trimmed_len, 8);
    name_arr[..copy_len].copy_from_slice(&name[..copy_len]);

    Some(freertos_task_info {
        tcb_addr: tcb,
        name: name_arr,
        tcb_no: 0,
        top_of_stack: topOfStack,
        bottom_of_stack: stack,
        priority,
        state,
    })
}

/*
 *
 */
pub fn freertos_collect_information() -> &'static Vec<freertos_task_info> {
    let cache = get_cache();
    if !cache.dirty {
        bmplog!("freertos_collect_information: returning cached data\n");
        return &cache.data;
    }

    let symbol = get_symbols();
    if !symbol.running {
        cache.dirty = false;
        return &cache.data;
    }

    // Read pcCurrentTcb
    let mut data: [u32; 1] = [0];
    if !bmp_read_mem32(get_current_tcb_address(), &mut data) {
        bmpwarning!("cannot read value of pxCurrentTCB\n");
        cache.dirty = false;
        return &cache.data;
    }
    // pxCurrentTCB
    let current = data[0];

    // add current TCB as number 1
    if let Some(mut x) = read_tcb(current, map_state[0]) {
        let tid = get_hashtcb().get(x.tcb_addr);
        x.tcb_no = tid;
        cache.data.push(x);
    }
    // read other lists
    bmplog!("---- scanning ----\n");
    for index in 1..5 {
        bmplog!(" parsing list {}:\n", index);
        for i in freertos_crawl_list(symbol.addresses[index].unwrap()) {
            bmplog!(
                "\t\t found TCB 0x{:x} indexed as {}\n",
                i,
                FreeRTOSSymbolName[index]
            );
            if i != current {
                // read the task info for each of the TCBs
                if let Some(mut x) = read_tcb(i, map_state[index]) {
                    let tid = get_hashtcb().get(x.tcb_addr);
                    x.tcb_no = tid;
                    cache.data.push(x);
                }
            } else {
                bmplog!("\t\t ==current, skipping\n");
            }
        }
    }
    bmplog!("---- scanning done ----\n");
    bmplog!("---- dumping ----\n");
    for t in &cache.data {
        t.print_tcb();
    }
    bmplog!("---- dumping done----\n");

    cache.dirty = false;
    &cache.data
}
/*
 *
 */
pub fn get_threads() -> Vec<u32> {
    bmplog!("get_threads\n");
    let mut output: Vec<u32> = Vec::new();
    let symbol = get_symbols();
    if !symbol.running {
        bmplog!("fos not running \n");
        return output;
    }
    let t = freertos_collect_information();
    for i in t {
        output.push(i.tcb_no);
    }
    output
}
/*
 *
 */
pub fn get_current_thread_id() -> Option<u32> {
    bmplog!("get_current_thread\n");
    let symbol = get_symbols();
    if !symbol.running {
        return None;
    }
    let t = freertos_collect_information();
    if t.is_empty() {
        bmplog!("invalid info\n");
        return None;
    }
    // lookup which one is current thread
    let tcb = get_pxCurrentTCB()?;
    for i in t {
        if i.tcb_addr == tcb {
            return Some(i.tcb_no);
        }
    }
    None
}

/*
 *
 */
pub fn get_tcb_info_from_id(id: u32) -> Option<freertos_task_info> {
    bmplog!("get_tcb_info_from\n");
    let t = freertos_collect_information();
    if t.is_empty() {
        bmplog!("invalid info\n");
        return None;
    }

    for r in t.iter() {
        r.print_tcb();
        if r.tcb_no == id {
            return Some(r.clone());
        }
    }
    bmplog!("Not found!\n");
    None
}

/*
 * \fn return a copy of pxCurrentTCB
 */
pub fn get_pxCurrentTCB() -> Option<u32> {
    bmplog!("get_pxCurrentTCB\n");
    let px_adr = get_current_tcb_address();
    let mut data: [u32; 1] = [0; 1];
    if !bmp_read_mem32(px_adr, &mut data[0..1]) {
        bmplog!("read error at 0x{:x}\n", px_adr);
        return None;
    }
    Some(data[0])
}
/*
 *
 */
pub fn set_pxCurrentTCB(tcb: u32) -> bool {
    bmplog!("set_pxCurrentTCB\n");
    let px_adr = get_current_tcb_address();
    let mut data: [u32; 1] = [0; 1];
    data[0] = tcb;
    bmp_write_mem32(px_adr, &data)
}
/*
 *
 *
 */
pub fn freertos_is_thread_present(thread_id: u32) -> bool {
    let new_info = get_tcb_info_from_id(thread_id);
    if new_info.is_some() {
        return true;
    }
    bmplog!("thread {} not present\n", thread_id);
    false
}
/*
 *
 *
 *
 */
pub fn freertos_switch_task(thread_id: u32) -> bool {
    // if we cant switch no need to go further
    bmplog!("switch_task\n");
    if !os_can_switch() {
        bmplog!("thread cannot switch ");
        return false;
    }
    let new_info = get_tcb_info_from_id(thread_id);
    // read new tcb
    let new_tcb = match new_info {
        None => {
            bmplog!("no tcb info for thread {} \n", thread_id);
            return false;
        }
        Some(x) => x,
    };
    // read old tcb, exit if it is actually the same as the new one
    let old_current_tcb_adr = match get_pxCurrentTCB() {
        None => {
            bmplog!("no tcb info for current TCB\n");
            return false;
        }
        Some(x) => {
            if x == new_tcb.tcb_addr {
                bmplog!("already at the right thread, nothing to do\n");
                return true;
            }
            x
        }
    };

    // let's switch
    bmplog!("switching...\n");
    let old_stack = freertos_switch_task_action(new_tcb.top_of_stack);
    if old_stack == 0 {
        bmpwarning!("freertos_switch_task: switch action failed (old_stack=0)\n");
        return false;
    }
    // write new top of stack
    let item: [u32; 1] = [old_stack];
    bmp_write_mem32(old_current_tcb_adr, &item);

    // switch to that thread..
    set_pxCurrentTCB(new_tcb.tcb_addr);
    true
}
// EOF