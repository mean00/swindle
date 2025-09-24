/*
 *
 *  This handles a very basic ARM only software breakpoints
 *
 *
 */
#![allow(static_mut_refs)]

//use core::ptr::addr_of_mut;

use alloc::vec::Vec;
use core::mem::MaybeUninit;

crate::setup_log!(false);
use crate::bmp;
use crate::bmplog;

const ARM_BREAKPOINT_OPCODE: [u8; 2] = [0x00u8, 0xbeu8];
const RISCV_BREAKPOINT_OPCODE: [u8; 2] = [0x02u8, 0x90u8];

struct address_old_opcode {
    address: u32,
    old_opcode: [u8; 4],
}
//
//
//
struct list_of_breakpoints {
    breakpoint: Vec<address_old_opcode>, // address, old opcode
}
//
//
//
impl list_of_breakpoints {
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
static mut lsw: list_of_breakpoints = list_of_breakpoints {
    breakpoint: Vec::new(),
};
static mut lmw: list_of_breakpoints = list_of_breakpoints {
    breakpoint: Vec::new(),
};
const MAX_SECTOR_SIZE: u32 = 256; // must be a power of 2!
static mut temp_buffer: MaybeUninit<[u8; MAX_SECTOR_SIZE as usize]> = MaybeUninit::uninit();

fn get_temp_buffer(size: u32) -> &'static mut [u8] {
    if size > MAX_SECTOR_SIZE {
        panic!("sector too big");
    }
    unsafe { &mut temp_buffer.assume_init_mut()[0..(size as usize)] }
}

//
//
//
fn get_list_sw_ref() -> &'static mut list_of_breakpoints {
    unsafe { &mut lsw }
}
fn get_list_mw_ref() -> &'static mut list_of_breakpoints {
    unsafe { &mut lmw }
}
/*
*
*/
pub fn clear_sw_breakpoint() {
    get_list_sw_ref().breakpoint.clear();
}
/*
*
*/
pub fn clear_mw_breakpoint() {
    get_list_mw_ref().breakpoint.clear();
}
/*
 *
 *
 */
fn patch_code(offset: u32, code: &mut [u8]) {
    let offset: usize = offset as usize;
    if !bmp::bmp_is_riscv() {
        code[offset] = ARM_BREAKPOINT_OPCODE[0];
        code[offset + 1] = ARM_BREAKPOINT_OPCODE[1];
    } else {
        code[offset] = RISCV_BREAKPOINT_OPCODE[0];
        code[offset + 1] = RISCV_BREAKPOINT_OPCODE[1];
    }
}

/*
 *
 */
pub fn add_sw_breakpoint(address: u32, _len: u32) -> bool {
    if get_list_sw_ref().is_present(address) {
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
    if !(bmp::bmp_read_mem(aligned_address, &mut breakpoint.old_opcode)) {
        bmplog!("cant read old sw value\n");
        return false;
    }
    // put new opcode
    let mut new_opcode: [u8; 4] = breakpoint.old_opcode;
    let offset: usize = (address & 2) as usize;
    patch_code(offset as u32, &mut new_opcode);
    if !bmp::bmp_mem_write(aligned_address, &new_opcode) {
        bmplog!("cant write new sw value\n");
        return false;
    }
    // add to list
    get_list_sw_ref().breakpoint.push(breakpoint);
    true
}
/*
 *
 */
/*
 *
 */
pub fn remove_sw_breakpoint(address: u32, _len: u32) -> bool {
    loop {
        let r = get_list_sw_ref().lookup_index(address);
        match r {
            None => return true, // all done
            Some(index) => {
                // put back the old code
                let aligned_address: u32 = address & 0xfffffffcu32;
                if !bmp::bmp_mem_write(aligned_address, &get_list_sw_ref().at(index).old_opcode) {
                    bmplog!("cant restore old sw value\n");
                }
                // remove X from list
                get_list_sw_ref().breakpoint.remove(index);
            }
        };
    }
}

/*
 *
 */
pub fn add_mw_breakpoint(address: u32) -> bool {
    if get_list_mw_ref().is_present(address) {
        // already done
        return true;
    }
    // Read old opcode
    let mut breakpoint = address_old_opcode {
        address,
        old_opcode: [0, 0, 0, 0],
    };
    // store breakpoint
    let page_size = bmp::bmp_get_mw_page_size();
    let aligned_address: u32 = address & !(page_size - 1);
    let offset: u32 = address - aligned_address;
    let offset_u: usize = offset as usize;
    let bf = get_temp_buffer(page_size);
    if !(bmp::bmp_read_mem(aligned_address, bf)) {
        bmplog!("cant read old sw value\n");
        return false;
    }
    // put new opcode
    breakpoint.old_opcode[0] = bf[offset_u];
    breakpoint.old_opcode[1] = bf[offset_u + 1];
    patch_code(offset, bf);
    if !bmp::bmp_overwrite_flash(aligned_address, bf) {
        bmplog!("cant write new sw value\n");
        return false;
    }
    // add to list
    get_list_mw_ref().breakpoint.push(breakpoint);
    true
}
/*
 *
 */
pub fn remove_mw_breakpoint(address: u32) -> bool {
    let index = match get_list_mw_ref().lookup_index(address) {
        None => return true, // all done
        Some(index) => index,
    };
    // put back the old code
    let page_size = bmp::bmp_get_mw_page_size();
    let aligned_address: u32 = address & !(page_size - 1);
    let offset = address - aligned_address;
    let offset_u: usize = offset as usize;
    let bf = get_temp_buffer(page_size);
    if !(bmp::bmp_read_mem(aligned_address, bf)) {
        bmplog!("cant read old mw value\n");
        return false;
    }
    let bk = get_list_mw_ref().at(index);

    bf[offset_u] = bk.old_opcode[0];
    bf[offset_u + 1] = bk.old_opcode[1];
    if !bmp::bmp_overwrite_flash(aligned_address, bf) {
        bmplog!("cant restore old sw value\n");
    }
    // remove X from list
    get_list_mw_ref().breakpoint.remove(index);
    true
}

// EOF
