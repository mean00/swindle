/*
 *  Hold a hashMap so that a given tcb address always gives the same task id
 *   whatever freertos compilation options are.
 *
 */
use alloc::vec::Vec;
use core::ptr::addr_of_mut;
crate::setup_log!(false);
//use crate::gdb_print;
//use crate::{bmplog, bmpwarning};
crate::gdb_print_init!();
use crate::bmp;

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
    gdb_println!("Hash index = ", Hex(info.index));
    for i in &info.list {
        let val: u32 = i.tcb;
        let id: u32 = i.tid;
        gdb_println!("------------ID: ", Hex(id), "-----------");
        gdb_print!("TCB:  ", Hex(val));
        bmp::bmp_read_mem32(val, &mut regs[0..1]);
        let stack: u32 = regs[0];
        gdb_println!("SP: ", Hex(stack));
        // Read the regs
        bmp::bmp_read_mem32(stack, &mut regs);
        #[allow(clippy::needless_range_loop)]
        for r in 0..4 {
            gdb_print!(" R", r);
            gdb_println!(" = 0x", Hex(regs[r + 8]));
        }
        gdb_print!("\n");
        #[allow(clippy::needless_range_loop)]
        for r in 4..8 {
            gdb_print!(" R", r);
            gdb_println!(" = 0x", Hex(regs[r]));
        }
        gdb_print!("\n");
        #[allow(clippy::needless_range_loop)]
        for r in 8..12 {
            gdb_print!(" R", r);
            gdb_println!(" = 0x", Hex(regs[r]));
        }
        gdb_print!("\n");
        gdb_println!(" PC = 0x", Hex(regs[13]));
        gdb_println!(" LR = 0x", Hex(regs[14]));
        gdb_println!(" xps = 0x", Hex(regs[15]));
    }
}
