
use crate::rn_bmp_cmd_c;
use cty;
use core::ptr::null;
use core::ptr::null_mut;
use alloc::vec::Vec;
pub enum mapping
{
    FLASH=0,
    RAM=1,
}

pub struct MemoryBlock
{
    start_address   : u32,
    length          : u32
}

pub fn bmp_get_mapping(map : mapping) -> Vec<MemoryBlock>
{
    if !bmp_attached()
    {
        let r : Vec<MemoryBlock> = Vec::new();
        return r;
    }
    let r : Vec<MemoryBlock> = Vec::new();

    return r;
}
pub fn swdp_scan() -> bool
{
    unsafe 
    {
        if rn_bmp_cmd_c::cmd_swdp_scan( null(), 0,null_mut() as *mut *const i8)!=0
        {            
            return true;
        }
        return false;
    }
}

pub fn bmp_attached() -> bool
{
    unsafe 
    {
        if rn_bmp_cmd_c::bmp_attached_c()!=0
        {            
            return true;
        }
        return false;
    }
}
pub fn bmp_attach(target : u32) -> bool
{
    unsafe 
    {
        if rn_bmp_cmd_c::bmp_attach_c(target)!=0
        {            
            return true;
        }
        return false;
    }
}
