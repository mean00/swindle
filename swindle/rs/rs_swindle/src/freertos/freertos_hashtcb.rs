/*
 *  Hold a hashMap so that a given tcb address always gives the same task id
 *   whatever freertos compilation options are.
 *
 */
use alloc::vec::Vec;
use core::ptr::addr_of_mut;
crate::setup_log!(false);
use crate::gdb_print;
use crate::{bmplog, bmpwarning};
crate::gdb_print_init!();

pub struct tcb_to_tid {
    tcb: u32,
    tid: u32,
}
pub struct hashed_tcb {
    list: Vec<tcb_to_tid>,
    index: u32,
}
/*
 *
 */
impl hashed_tcb {
    /*
     *
     */
    pub fn new() -> Self {
        hashed_tcb {
            list: Vec::new(),
            index: 1,
        }
    }
    /*
     *
     */
    pub fn clear(&mut self) {
        self.list.clear();
        self.index = 1;
    }
    /*
     *
     */
    pub fn get(&mut self, tcb: u32) -> u32 {
        for i in &self.list {
            if i.tcb == tcb {
                return i.tid;
            }
        }
        // not found create a new one
        let new_entry: tcb_to_tid = tcb_to_tid {
            tcb,
            tid: self.index,
        };
        self.list.push(new_entry);
        self.index += 1;
        self.index - 1 // the last entry
    }
}

//--

static mut tcb_hashmap: Option<hashed_tcb> = None;
/*
 *
 */
pub fn get_hashtcb() -> &'static mut hashed_tcb {
    unsafe {
        if tcb_hashmap.is_none() {
            tcb_hashmap = Some(hashed_tcb::new());
        }
        match &mut *addr_of_mut!(tcb_hashmap) {
            Some(x) => x,
            None => panic!("hashap"),
        }
    }
}
/*
 *
 */
pub fn dump_hash_info() {
    let mut regs: [u32; 16] = [0; 16];
    let info = get_hashtcb();
    gdb_print!("Hash index = {}\n", info.index);
    for i in &info.list {
        let val: u32 = i.tcb;
        let id: u32 = i.tid;
        gdb_print!("------------ID: {:x}-----------\n", id);
        gdb_print!("TCB: {:x} ", val);
        crate::bmp::bmp_read_mem32(val, &mut regs[0..1]);
        let stack: u32 = regs[0];
        gdb_print!("SP: {:x} \n", stack);
        // Read the regs
        crate::bmp::bmp_read_mem32(stack, &mut regs);
        #[allow(clippy::needless_range_loop)]
        for r in 0..4 {
            gdb_print!(" R{} = 0x{:x}", r, regs[r + 8]);
        }
        gdb_print!("\n");
        #[allow(clippy::needless_range_loop)]
        for r in 4..8 {
            gdb_print!("    R{} = 0x{:x}", r, regs[r]);
        }
        gdb_print!("\n");
        #[allow(clippy::needless_range_loop)]
        for r in 8..12 {
            gdb_print!("    R{} = 0x{:x}", r, regs[r]);
        }
        gdb_print!("\n");
        gdb_print!(" PC = 0x{:x}\n", regs[13]);
        gdb_print!(" LR = 0x{:x}\n", regs[14]);
        gdb_print!(" xps = 0x{:x}\n", regs[15]);
    }
}
