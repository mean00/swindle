
use crate::rn_bmp_cmd_c;
use cty;
use core::ptr::null;
use core::ptr::null_mut;

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


pub fn bmp_attach(target : u32) -> bool
{
    unsafe 
    {
        if rn_bmp_cmd_c::bmp_attach(target)!=0
        {            
            return true;
        }
        return false;
    }
}
