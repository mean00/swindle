/*
 *
 *  This handles a very basic ARM only software breakpoints
 *
 *
 */
//
#![allow(static_mut_refs)]

use crate::parsing_util::ascii_string_to_u32;
use core::ptr::addr_of_mut;

use crate::bmp;

use alloc::vec;
use alloc::vec::Vec;

crate::setup_log!(true);
use crate::{bmplog, bmpwarning};

const ARM_BREAKPOINT_OPCODE0: u8 = 0x00u8;
const ARM_BREAKPOINT_OPCODE1: u8 = 0xbeu8;

struct address_old_opcode {
    address: u32,
    old_opcode: [u8; 4],
}
//
//
//
struct list_of_sw_breakpoints {
    breakpoint: Vec<address_old_opcode>, // address, old opcode
}
//
//
//
impl list_of_sw_breakpoints {
    //
    //
    //
    fn lookup_index(&self, adr: u32) -> Option<usize> {
        for i in 0..self.breakpoint.len() {
            let this_adr = self.breakpoint[i].address;
            if this_adr == adr {
                return Some(i);
            }
        }
        None
    }
    //
    //
    //
    fn is_present(&self, adr: u32) -> bool {
        for item in self.breakpoint.iter() {
            if item.address == adr {
                return true;
            }
        }
        false
    }
    fn at(&self, dex: usize) -> &address_old_opcode {
        &self.breakpoint[dex]
    }
}

//
//
//
static mut lsw: list_of_sw_breakpoints = list_of_sw_breakpoints {
    breakpoint: Vec::new(),
};
//
//
//
fn get_list_ref() -> &'static mut list_of_sw_breakpoints {
    unsafe { &mut lsw }
}

/**
 *
 */
pub fn add_sw_breakpoint(address: u32, _len: u32) -> bool {
    if get_list_ref().is_present(address) {
        // already done
        return true;
    }
    // Read old opcode
    let mut breakpoint = address_old_opcode {
        address,
        old_opcode: [0, 0, 0, 0],
    };
    // store breakpoint
    let aligned_address: u32 = address & 0xfffffffcu32;
    if !(crate::bmp::bmp_read_mem(aligned_address, &mut breakpoint.old_opcode)) {
        bmplog!("cant read old sw value\n");
        return false;
    }
    // put new opcode
    let mut new_opcode: [u8; 4] = breakpoint.old_opcode;
    let offset: usize = (address & 2) as usize;
    new_opcode[offset + 0] = ARM_BREAKPOINT_OPCODE0;
    new_opcode[offset + 1] = ARM_BREAKPOINT_OPCODE1;
    if !crate::bmp::bmp_mem_write(aligned_address, &new_opcode) {
        bmplog!("cant write new sw value\n");
        return false;
    }
    // add to list
    get_list_ref().breakpoint.push(breakpoint);
    true
}
/**
 *
 */
pub fn remove_sw_breakpoint(address: u32, _len: u32) -> bool {
    loop {
        let r = get_list_ref().lookup_index(address);
        match r {
            None => return true, // all done
            Some(index) => {
                // put back the old code
                let aligned_address: u32 = address & 0xfffffffcu32;
                if !crate::bmp::bmp_mem_write(aligned_address, &get_list_ref().at(index).old_opcode)
                {
                    bmplog!("cant restore old sw value\n");
                }
                // remove X from list
                get_list_ref().breakpoint.remove(index);
            }
        };
    }
}
// EOF
