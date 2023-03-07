
use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::util::hex_to_u8s;

use crate::encoder::encoder;
use super::{CommandTree,exec_one};
use crate::packet_symbols::INPUT_BUFFER_SIZE;

use super::mon::{_swdp_scan};
use crate::bmp::bmp_attached;
use crate::bmp::bmp_get_mapping;
use crate::bmp::MemoryBlock;
use crate::bmp::mapping::{ FLASH,RAM};


use numtoa::NumToA;

pub fn _X(_tokns : &Vec<&str>) -> bool
{
    if !crate::bmp::bmp_attached()
    {
        encoder::simple_send("E01");    
        return true;
    }
    encoder::simple_send("OK");    
    return true;
}
