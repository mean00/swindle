use crate::parsing_util;
use crate::encoder::encoder;
use alloc::vec::Vec;
use crate::bmp::{bmp_read_mem,bmp_read_mem32};

crate::setup_log!(true);
use crate::{bmplog, bmpwarning};

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
pub struct FreeRTOSSymbols
{
    valid : bool,    
    index : usize,
    symbols : [u32;5],    
}

/**
 * \brief : ask gdb for pxCurrentTCB address
 */

static mut freeRtosSymbols: FreeRTOSSymbols = FreeRTOSSymbols {
    valid: false,
    index : 0,
    symbols : [0,0,0,0,0],
};
unsafe fn ask_for_next_symbol() -> bool
{
    let mut e=encoder::new();
    e.begin();
    e.add("qSymbol:");
    e.hex_and_add(FreeRTOSSymbolName[freeRtosSymbols.index]);
    e.end();
    true

}
/**
 * \fn freertos_crawl_list
 * \brief crawl a list of TCBs and popup the individual PCB, the out is a pointer disguised as a u32
 */
fn freertos_crawl_list(address: u32) -> Vec<u32>
{
    let mut v: Vec<u32>=Vec::new();    
    let mut list_header : [u32;3]=[0,0,0];

    // Read the list head
    if !bmp_read_mem32(address, &mut list_header) 
    {
        bmplog!("Fail to parse list at address 0x{:x}\n",address);
        return v;
    }
   
    
    let mut count = list_header[0];
    let mut next = list_header[1];
    let mut end = list_header[2];
    
    let mut items : [u32;5]=[0,0,0,0,0];

    for i in 0..count
    {
        // Read item
        // 0 number
        // 1 next
        // 2 previous
        // 3 owner <- tcv
        // 4 container
        if !bmp_read_mem32(address, &mut items) 
        {
            bmplog!("Fail to parse list at address 0x{:x}\n",next);
            return v;
        }       
        next=items[1];
        // owner is a StateListeItem
        if !bmp_read_mem32(items[3], &mut items) 
        {
            bmplog!("Fail to parse list at address 0x{:x}\n",next);
            return v;
        }
        v.push(items[3]); // go back 4 
        
    }
    return v;
}
/**
 * 
 */
pub unsafe fn freertos_collect_information() -> bool
{    
    if !freeRtosSymbols.valid   
    {
        return false;
    }
    let mut index : usize;
    let list_of_tcb : Vec<u32> = Vec::new();
    for index in 1..5
    {
        let this_list = freertos_crawl_list( freeRtosSymbols.symbols[index]);
        // Merge into master list
        bmpwarning!(" {} \n", FreeRTOSSymbolName[index]);
        for i in this_list
        {
            bmpwarning!("\t {:x}  \n", i);
        }
        
    }
    true
}
/**
 * 
 */
pub unsafe fn q_freertos_symbols(args: &[&str]) -> bool {

    unsafe {
    if freeRtosSymbols.valid
    {
        encoder::reply_ok();
        return true;
    }}
    if args.len()!=2    {
        bmpwarning!("Incorrect reply size {}",args.len());
        encoder::reply_e01();
        return true;
    }
    // is it an empty one ?, if so ask for pxCurrentTcb
    if args[0].is_empty() && args[1].is_empty()    {
        
        freeRtosSymbols.index=0;
        freeRtosSymbols.valid=false;    
        return ask_for_next_symbol();
    }
    // Ok what is the symbol ?
    let mut symbol : [u8;32] = [0;32];         
    let clear_text  = parsing_util::ascii_hex_string_to_str(args[1], &mut symbol);
    if let Ok(text ) = clear_text   {
        if text.eq(FreeRTOSSymbolName[freeRtosSymbols.index])
        {
            if args[0].is_empty()
            {
                bmpwarning!("cannot find symbol {}\n", text);
                encoder::reply_ok();
                return true;       
            }else {// next            
                let  address: u32 = parsing_util::ascii_string_to_u32(args[0]);
                bmplog!("Found symbol {} : 0x{:x}\n", FreeRTOSSymbolName[freeRtosSymbols.index], address);
                freeRtosSymbols.symbols[freeRtosSymbols.index] = address;
                freeRtosSymbols.index+=1;
                if freeRtosSymbols.index==5 
                {
                    bmplog!("Got all symbols\n");
                    freeRtosSymbols.valid=true;
                    encoder::reply_ok();
                    return true;        
                }
                return ask_for_next_symbol();   
            }
        }  else    {
            bmpwarning!("Inconsistent reply for index {}, reply is {}\n",freeRtosSymbols.index, text);
        }                              
    }
    bmpwarning!("Invalid qsymbol reply\n");
    false
}
/**
 * \fn return a copy of pxCurrentTCB
 */
pub fn get_pxCurrentTCB() -> Option <u32>
{
    unsafe {  freertos_collect_information(); }

  None
}



// EOF
