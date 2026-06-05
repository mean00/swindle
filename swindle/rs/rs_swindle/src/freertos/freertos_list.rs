/*
    https://aosabook.org/en/v2/freertos.html
*/

use alloc::vec::Vec;

use crate::bmp::bmp_read_mem32;
use crate::freertos::freertos_symbols::{get_symbols, FreeRTOSDebugOffsets};
crate::setup_log!(false);
crate::gdb_print_init!();
//use crate::bmplog;

/// Default hardcoded list offsets used as fallback.
const FALLBACK_LIST_OFFSETS: FreeRTOSDebugOffsets = FreeRTOSDebugOffsets {
    magic: 0,
    list_size: 0,
    offset_list_item_next: 4,
    offset_list_item_owner: 12,
    offset_list_number_of_item: 0,
    offset_list_index: 4,
    nb_of_priorities: 16,
    mpu_enabled: 0,
    max_task_name_len: 16,
    offset_task_name: 52,
    offset_task_num: 68,
};

fn get_list_offsets() -> FreeRTOSDebugOffsets {
    get_symbols().debug_offsets.unwrap_or(FALLBACK_LIST_OFFSETS)
}

/**
 * \fn freertos_crawl_list
 * \brief returns a list of items, in out case TCB *
 */
pub fn freertos_crawl_list(address: u32) -> Vec<u32> {
    let off = get_list_offsets();
    let mut v: Vec<u32> = Vec::new();
    let mut list_header: [u32; 5] = [0, 0, 0, 0, 0];

    // Read the list head
    //  0 #of items
    //  1 *pIndex = last item
    //listEnd
    //  2 item value (ffff)
    //  3 *pxNext = start of list <= beignning of the list
    //  4 *previous
    //  5 *owner
    //  6 *container
    if !bmp_read_mem32(address, &mut list_header) {
        bmplog!("Fail to parse list at address 0x{:x}\n", address);
        return v;
    }
    let count = list_header[off.offset_list_number_of_item as usize / 4];

    if count > 20
    // if it's unresonnable it's garbage
    {
        bmplog!("Unreasonnably large list\n");
        return v;
    }

    let mut next = list_header[off.offset_list_index as usize / 4];

    let mut items: [u32; 5] = [0, 0, 0, 0, 0];

    // The list is circular with an end marker (xListEnd) that has
    // xItemValue == portMAX_DELAY (0xFFFFFFFF). We must skip it.
    // We iterate until we've collected `count` real items, or up to
    // `count + 1` iterations to account for starting at xListEnd.
    const PORT_MAX_DELAY: u32 = 0xFFFFFFFF;
    let mut checked: u32 = 0;

    for _i in 0..=(count + 1) {
        if checked >= count {
            break;
        }
        // Read item
        // 0 number
        // 1 *next
        // 2 *previous
        // 3 *owner <- tcb
        // 4 *container
        if !bmp_read_mem32(next, &mut items) {
            bmplog!("Fail to parse list at address 0x{:x}\n", next);
            return v;
        }
        next = items[off.offset_list_item_next as usize / 4];
        // Skip the end marker (xListEnd) — it has no valid owner
        if items[0] == PORT_MAX_DELAY {
            continue;
        }
        checked += 1;
        let tcb = items[off.offset_list_item_owner as usize / 4];
        v.push(tcb);
    }
    v
}
// EOF
