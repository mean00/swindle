use crate::parsing_util;
use crate::encoder::encoder;

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
            }else // next
            {
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
        }  else
        {
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
  None
}



// EOF
