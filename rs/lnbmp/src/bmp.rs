
use crate::rn_bmp_cmd_c;
use cty;
use core::ptr::null;
use core::ptr::null_mut;
use alloc::vec::Vec;
use core::ffi::CStr;
pub enum mapping
{
    FLASH=0,
    RAM=1,
}

pub struct MemoryBlock
{
    pub start_address   : u32,
    pub length          : u32,
    pub block_size      : u32,
}
pub fn bmp_register_description() -> &'static str
{
    // 
    unsafe {
    match( CStr::from_ptr(rn_bmp_cmd_c::bmp_target_description_c()).to_str() ) 
    {
        Ok(x) => x,
        Err(_y) => "",
    }
  }
}
pub fn bmp_read_registers() -> Vec<u32>
{
    if !bmp_attached()
    {
        let r : Vec<u32> = Vec::new();
        return r;
    }
    let mut r : Vec<u32> = Vec::new();    
    let n= unsafe { rn_bmp_cmd_c::bmp_registers_count_c()  };
    for i in 0..n
    {
        let mut val: u32 = 0;
        unsafe {
            if rn_bmp_cmd_c::bmp_read_register_c(i, &mut val as *mut u32) != 0
            {
                r.push(val);
            }
        }        
    }
    return r;
}
pub fn bmp_get_mapping(map : mapping) -> Vec<MemoryBlock>
{
    if !bmp_attached()
    {
        let r : Vec<MemoryBlock> = Vec::new();
        return r;
    }
    let mut r : Vec<MemoryBlock> = Vec::new();
    let count: usize;
    let imap = map as u32;
    unsafe {
    count = rn_bmp_cmd_c::bmp_map_count_c(imap) as usize;
    };
    if count!=0
    {
        unsafe {
        let mut start :         u32 =0 ;
        let mut size :          u32 =0;
        let mut block_size :    u32 =0;
        for i in 0..count
        {
            
                if  rn_bmp_cmd_c::bmp_map_get_c(imap, i as u32,
                            &mut start as *mut u32,
                            &mut size as *mut u32,
                            &mut block_size as *mut u32) !=0
                {
                    r.push( MemoryBlock{start_address : start as u32, length : size as u32, block_size : block_size as u32});
                }
        }
      }
    }    
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
