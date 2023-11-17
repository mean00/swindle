use alloc::vec::Vec;

use crate::bmp::{bmp_read_mem,bmp_read_mem32};
crate::setup_log!(false);
crate::gdb_print_init!();
use crate::{bmplog, bmpwarning};
/**
 * \fn freertos_crawl_list
 */
pub fn freertos_crawl_list(address : u32) -> Vec<u32>
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
// EOF