use crate::bmp::{bmp_read_mem, bmp_read_mem32, bmp_write_mem32};
use alloc::vec::Vec;

use crate::freertos::freertos_hashtcb::get_hashtcb;
use crate::freertos::freertos_list::{freertos_crawl_list, freertos_replace_in_list};

use crate::freertos::freertos_symbols::{get_current_tcb_address, get_symbols};
use crate::freertos::freertos_trait::freertos_task_state;

use crate::freertos::{freertos_switch_task_action, freertos_task_info, os_can_switch};

crate::setup_log!(false);
use crate::{bmplog, bmpwarning};

const OFFSET_TO_SIZE: u32 = 44;

const map_state: [freertos_task_state; 5] = [
    freertos_task_state::running,
    freertos_task_state::suspended,
    freertos_task_state::blocked,
    freertos_task_state::blocked,
    freertos_task_state::ready,
];

enum freeRtosSymbolIndex {
    pxCurrentTCB = 0,
    xSuspendedTaskList = 1,
    xDelayedTaskList1 = 2,
    xDelayedTaskList2 = 3,
    pxReadyTasksLists = 4,
}
const FreeRTOSSymbolName: [&str; 5] = [
    "pxCurrentTCB",
    "xSuspendedTaskList",
    "xDelayedTaskList1",
    "xDelayedTaskList2",
    "pxReadyTasksLists",
];

//--
/*
 *
 */

fn read_tcb(tcb: u32, state: freertos_task_state) -> Option<freertos_task_info> {
    let mut data: [u32; 16] = [0; 16];
    if !bmp_read_mem32(tcb, &mut data[0..1]) {
        bmpwarning!("cannot read TCB\n");
        return None;
    }
    let topOfStack: u32 = data[0];
    if !bmp_read_mem32(tcb + OFFSET_TO_SIZE, &mut data[0..2]) {
        bmpwarning!("cannot read TCB\n");
        return None;
    }
    let priority = data[0];
    let stack = data[1];

    let mut name: [u8; 8] = [b' '; 8];
    if !bmp_read_mem(tcb + OFFSET_TO_SIZE + 8, &mut name) {
        bmpwarning!("cannot read name\n");
        return None;
    }
    Some(freertos_task_info {
        tcb_addr: tcb,
        name,
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
pub fn freertos_collect_information() -> Vec<freertos_task_info> {
    let mut output: Vec<freertos_task_info> = Vec::new();
    let symbol = get_symbols();
    if !symbol.running {
        return output;
    }

    // Read pcCurrentTcb
    let mut data: [u32; 1] = [0];
    if !bmp_read_mem32(get_current_tcb_address(), &mut data) {
        bmpwarning!("cannot read value of pxCurrentTCB\n");
        return output;
    }
    // pxCurrentTCB
    let current = data[0];

    // add current TCB as number 1
    if let Some(mut x) = read_tcb(current, map_state[0]) {
        let tid = get_hashtcb().get(x.tcb_addr);
        x.tcb_no = tid;
        output.push(x);
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
                    output.push(x);
                }
            } else {
                bmplog!("\t\t ==current, skipping\n");
            }
        }
    }
    bmplog!("---- scanning done ----\n");
    bmplog!("---- dumping ----\n");
    for t in &output {
        t.print_tcb();
    }
    bmplog!("---- dumping done----\n");
    output
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

    for r in t.into_iter() {
        r.print_tcb();
        if r.tcb_no == id {
            return Some(r);
        }
    }
    bmplog!("Not found!\n");
    None
    //t.into_iter().find(|i| i.tcb_no == id)
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
    // write new top of stack
    let item: [u32; 1] = [old_stack];
    bmp_write_mem32(old_current_tcb_adr, &item);
    let symbol = get_symbols();
    // swap old and new tcb
    for index in 1..5 {
        bmplog!(" replacing in  list {}:\n", index);
        freertos_replace_in_list(
            symbol.addresses[index].unwrap(),
            new_tcb.tcb_addr,
            old_current_tcb_adr,
        );
    }

    // switch to that thread..
    set_pxCurrentTCB(new_tcb.tcb_addr);
    true
}
// EOF
