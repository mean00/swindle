

use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::bmp;
use crate::encoder::encoder;

pub fn _swdp_scan(_tokns : &Vec<&str>) -> bool
{
    glog("swdp_scan");
    if bmp::swdp_scan()
    {
        glog("success!");
        encoder::reply_ok();
        return true;
    }
    glog("fail!");
    return false;
    
}