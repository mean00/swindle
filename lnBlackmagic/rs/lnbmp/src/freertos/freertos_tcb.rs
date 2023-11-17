use crate::parsing_util;
use crate::encoder::encoder;
use alloc::vec::Vec;
use crate::bmp::{bmp_read_mem,bmp_read_mem32};

use crate::freertos::freertos_trait::{freertos_task_info,freertos_task_state};
use crate::freertos::freertos_symbols::get_symbols;
use crate::freertos::freertos_list::freertos_crawl_list;

crate::setup_log!(true);
use crate::{bmplog, bmpwarning};

const OFFSET_TO_SIZE : u32 = 44;

const map_state  : [freertos_task_state;5] = [ 
    freertos_task_state::running, 
    freertos_task_state::suspended, 
    freertos_task_state::blocked, 
    freertos_task_state::blocked, 
    freertos_task_state::ready
];


enum freeRtosSymbolIndex
{
    pxCurrentTCB = 0,
    xSuspendedTaskList = 1,
    xDelayedTaskList1= 2,
    xDelayedTaskList2= 3,
    pxReadyTasksLists=4,
}
const FreeRTOSSymbolName: [&str;5] = ["pxCurrentTCB",
                                    "xSuspendedTaskList",
                                    "xDelayedTaskList1",
                                    "xDelayedTaskList2",
                                    "pxReadyTasksLists"];
/**
 * 
 */


fn read_tcb(tcb : u32, state : freertos_task_state) -> Option<freertos_task_info>
{
    let mut data : [u32;16] = [0;16];
    if !bmp_read_mem32(tcb, &mut data[0..1]) 
    {
        bmpwarning!("cannot read TCB\n");
        return None;
    }
    let topOfStack : u32 = data[0];
    if !bmp_read_mem32(tcb+OFFSET_TO_SIZE, &mut data[0..2]) 
    {
        bmpwarning!("cannot read TCB\n");
        return None;
    }
    let priority = data[0];
    let stack = data[1];


    let mut name : [u8;8]  = [b' ';8];
    if !bmp_read_mem(tcb+OFFSET_TO_SIZE+8, &mut name) 
    {
        bmpwarning!("cannot read name\n");
        return None; 
    }
    Some( freertos_task_info {
        tcb_addr        : tcb,
                          name,
        tcb_no          : 0,    
        top_of_stack    : topOfStack,
        bottom_of_stack : stack,
                          priority,
                          state,
    }   )
}

/**
 * 
 */
pub fn freertos_collect_information() -> Vec<freertos_task_info>
{   
    let mut output: Vec<freertos_task_info>=Vec::new();     
    let symbol = get_symbols();
    if !symbol.valid   
    {
        return output;
    }
    
    // Read pcCurrentTcb
    let mut data : [u32;1] = [0];
    if !bmp_read_mem32(symbol.symbols[0], &mut data) 
    {
        bmpwarning!("cannot read value of pxCurrentTCB\n");
        return output;
    }
    // pxCurrentTCB
    let current = data[0];    
    // read other lists
    for index in 1..5
    {        
        for i in freertos_crawl_list( symbol.symbols[index])
        {
            bmplog!("\tTCB 0x{:x}\n",i);
            if i!=current
            {
                // read the task info for each of the TCBs
                if let Some(x) = read_tcb(i,map_state[index]) {
                    output.push(x);
                }
            }
        }
    }     
    // add current TCB
    if let Some(x) = read_tcb(current,map_state[0]) {
        output.push(x);
    }
    output
}

/**
 * \fn return a copy of pxCurrentTCB
 */
pub fn get_pxCurrentTCB() -> Option <u32>
{
   
  None
}



// EOF