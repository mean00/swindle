
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

fn ret_to_bool( ret : core::ffi::c_int) -> bool
{
    if ret!=0
    {
        return true;
    }
    false
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
    match  CStr::from_ptr(rn_bmp_cmd_c::bmp_target_description_c()).to_str()  
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
        ret_to_bool(  rn_bmp_cmd_c::cmd_swdp_scan( null(), 0,null_mut() as *mut *const i8))
    }
}

pub fn bmp_attached() -> bool
{
    unsafe 
    {
        ret_to_bool( rn_bmp_cmd_c::bmp_attached_c() )
    }
}
pub fn bmp_attach(target : u32) -> bool
{
    unsafe 
    {
        ret_to_bool(  rn_bmp_cmd_c::bmp_attach_c(target) )
    }
}
pub fn bmp_write_register(reg: u32, value: u32) -> bool
{
    unsafe {
        ret_to_bool( rn_bmp_cmd_c::bmp_write_reg_c(reg,value) )
    }
}
pub fn bmp_read_register(reg: u32) -> Option<u32>
{
    let mut val : u32 = 0;
    unsafe {
        let ptr : *mut u32  = &mut val;
        if rn_bmp_cmd_c::bmp_read_reg_c(reg,ptr)!=0
        {
            return Some(val);
        }
        return None;
    }
}
pub fn bmp_flash_erase(adr: u32, size: u32) -> bool
{
    unsafe {
        ret_to_bool( rn_bmp_cmd_c::bmp_flash_erase_c(adr,size) )
    }
}

pub fn bmp_flash_write(adr: u32, data : &[u8]) -> bool
{
    unsafe {
        let ptr  : * const u8 = data.as_ptr();
        ret_to_bool( rn_bmp_cmd_c::bmp_flash_write_c(adr, data.len() as u32,ptr) )
    }
}

pub fn bmp_flash_complete() -> bool
{
    unsafe {
        ret_to_bool( rn_bmp_cmd_c::bmp_flash_complete_c() )
    }
}

pub fn bmp_crc32( address : u32, length : u32) -> Option<u32>
{
    unsafe {
        let mut crc : u32 = 0;
        let  crc_ptr : *mut u32 = &mut crc;
        if rn_bmp_cmd_c::bmp_crc32_c(address, length, crc_ptr)!=0
        {
            return Some(crc);
        }
        return None;
    }
}

pub fn bmp_read_mem(address : u32, data : &mut [u8]) -> bool
{
    unsafe {        
        // mem_read_c returns flase if ok (WTF)
        !ret_to_bool( rn_bmp_cmd_c::bmp_mem_read_c(
            address, 
            data.len() as u32, 
            data.as_mut_ptr() as *mut u8) )
    }
}

pub fn bmp_reset_target() -> bool
{
    unsafe {
        ret_to_bool( rn_bmp_cmd_c::bmp_reset_target_c() )
    }
}
/*

typedef enum target_breakwatch {
	TARGET_BREAK_SOFT 0,
	TARGET_BREAK_HARD 1,
	TARGET_WATCH_WRITE 2,
	TARGET_WATCH_READ 3,
	TARGET_WATCH_ACCESS 4,
} target_breakwatch_e;

 */

pub fn  bmp_add_breakpoint(btype: u32, adr : u32 , len : u32) -> bool
{
    unsafe {
        ret_to_bool( rn_bmp_cmd_c::bmp_add_breakpoint_c(btype,adr,len) )
    }
}
pub fn  bmp_remove_breakpoint(btype: u32, adr : u32 , len : u32) -> bool
{
    unsafe {
        ret_to_bool( rn_bmp_cmd_c::bmp_remove_breakpoint_c(btype,adr,len))
    }
}
// resume go or step by step
pub fn bmp_halt_resume( step : bool )-> bool
{
    unsafe {
        let mut s : i32=0;
        if step
        {
            s=1;
        }
        ret_to_bool( rn_bmp_cmd_c::bmp_target_halt_resume_c(s))
    }
}

// EOF