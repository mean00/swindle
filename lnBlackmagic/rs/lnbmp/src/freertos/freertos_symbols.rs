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
 * \brief crawl a list of TCBs and popup the individual TCB.
 * /!\ Depending if MPU is enabled or not, the offset to data could be different!
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
    
    let count = list_header[0];
    let mut next = list_header[1];
    //let mut end = list_header[2];
    
    let mut items : [u32;5]=[0,0,0,0,0];

    for _i in 0..count
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

        let state_list_item = items[3];
        // owner is a StateListeItem
        // 0 Item
        // 1 next
        // 2 Prev
        // 3 owner <= the TCB is the actual owner
        if !bmp_read_mem32(state_list_item, &mut items) 
        {
            bmplog!("Fail to parse list at address 0x{:x}\n",next);
            return v;
        }        
        let top_of_stack = items[3];
        v.push( top_of_stack); 
    }
    v
}
/**
 * 
 */
const OFFSET_TO_SIZE : u32 = 44;

fn print_tcb(tcb : u32) -> bool
{
    let mut data : [u32;16] = [0;16];
    if !bmp_read_mem32(tcb, &mut data[0..1]) 
    {
        bmpwarning!("cannot read TCB\n");
        return false;
    }
    let topOfStack : u32 = data[0];
    if !bmp_read_mem32(tcb+OFFSET_TO_SIZE, &mut data[0..2]) 
    {
        bmpwarning!("cannot read TCB\n");
        return false;
    }
    let priority = data[0];
    let stack = data[1];


    let mut name : [u8;8]  = [b' ';8];
    if !bmp_read_mem(tcb+OFFSET_TO_SIZE+8, &mut name) 
    {
        bmpwarning!("cannot read name\n");
        return false; 
    }
    let name_as_string = unsafe { core::str::from_utf8_unchecked(&name) };

    bmplog!("Found thread {}: top of stack = {:x} priority = {:x} stack = {:x}\n",name_as_string, topOfStack, priority, stack);
    true
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
    //let mut index : usize;
    
    let mut list_of_tcb : [Vec<u32>;5]= [Vec::new(),Vec::new(),Vec::new(),Vec::new(),Vec::new()];

    // Read pcCurrentTcb
    let mut data : [u32;1] = [0];
    if !bmp_read_mem32(freeRtosSymbols.symbols[0], &mut data) 
    {
        bmpwarning!("cannot read value of pxCurrentTCB\n");
        return false;
    }
    // pxCurrentTCB
    list_of_tcb[0].push(data[0]);
    // read other lists
    for index in 1..5
    {        
        list_of_tcb [index] = freertos_crawl_list( freeRtosSymbols.symbols[index]);            
    }
    // ok we have all the TCBs
    for i in 0..5
    {
        bmplog!("List {}\n",i);
        for tcb in 0..list_of_tcb[i].len()
        {
            let tcb_adr = list_of_tcb[i][tcb];
            bmplog!("\tTCB 0x{:x}\n",tcb_adr);
            print_tcb(tcb_adr);
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
