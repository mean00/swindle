/*
    https://aosabook.org/en/v2/freertos.html
*/

use alloc::vec::Vec;

use crate::bmp::{bmp_read_mem,bmp_read_mem32};
crate::setup_log!(false);
crate::gdb_print_init!();
use crate::{bmplog, bmpwarning};
/**
 * \fn freertos_crawl_list
 * \brief returns a list of items, in out case TCB *
 */
pub fn freertos_crawl_list(address : u32) -> Vec<u32>
{
    let mut v: Vec<u32>=Vec::new();    
    let mut list_header : [u32;5]=[0,0,0,0,0];

    // Read the list head
    //  0 #of items 
    //  1 *pIndex = last item
    //listEnd
    //  2 item value (ffff)
    //  3 *pxNext = start of list <= beignning of the list
    //  4 *previous
    //  5 *owner
    //  6 *container
    if !bmp_read_mem32(address, &mut list_header) 
    {
        bmplog!("Fail to parse list at address 0x{:x}\n",address);
        return v;
    }
    let count = list_header[0];
    let mut next = list_header[3];

    let mut items : [u32;5]=[0,0,0,0,0];

    for _i in 0..count
    {
        // Read item
        // 0 number
        // 1 *next
        // 2 *previous
        // 3 *owner <- tcb
        // 4 *container
        if !bmp_read_mem32(next, &mut items) 
        {
            bmplog!("Fail to parse list at address 0x{:x}\n",next);
            return v;
        }       
        next=items[1];
        let tcb = items[3];        
        v.push( tcb);  
    }
    v
}
// EOF