/**
 *  Hold a hashMap so that a given tcb address always gives the same task id
 *   whatever freertos compilation options are.
 *
 */
use core::ptr::addr_of_mut;
use hashbrown::HashMap;
crate::setup_log!(false);
use crate::{bmplog, bmpwarning, gdb_print};
crate::gdb_print_init!();
pub struct hashed_tcb {
    hash: HashMap<u32, u32>,
    index: u32,
}
/**
 *
 */
impl hashed_tcb {
    /**
     *
     */
    pub fn new() -> Self {
        hashed_tcb {
            hash: HashMap::new(),
            index: 1,
        }
    }
    /**
     *
     */
    pub fn clear(&mut self) {
        self.hash.drain();
        self.index = 1;
    }
    /**
     *
     */
    pub fn get(&mut self, tcb: u32) -> u32 {
        let id: u32 = match self.hash.get(&tcb) {
            Some(x) => *x,
            None => {
                self.hash.insert(tcb, self.index);
                self.index += 1;
                self.index - 1
            }
        };
        id
    }
}

//--

static mut tcb_hashmap: Option<hashed_tcb> = None;
/**
 *
 */
pub fn get_hashtcb() -> &'static mut hashed_tcb {
    unsafe {
        if tcb_hashmap.is_none() {
            tcb_hashmap = Some(hashed_tcb::new());
        }
        match &mut *addr_of_mut!(tcb_hashmap) {
            Some(ref mut x) => x,
            None => panic!("hashap"),
        }
    }
}
/**
 *
 */
pub fn dump_hash_info() {
    let mut regs: [u32; 16] = [0; 16];
    let info = get_hashtcb();
    gdb_print!("Hash index = {}\n", info.index);
    for i in info.hash.keys() {
        let val: u32 = *(info.hash.get(i).unwrap());
        gdb_print!("------------ID: {:x}-----------\n", val);
        gdb_print!("TCB: {:x} ", *i);
        crate::bmp::bmp_read_mem32(*i, &mut regs[0..1]);
        let stack: u32 = regs[0];
        gdb_print!("SP: {:x} \n", stack);
        // Read the regs
        crate::bmp::bmp_read_mem32(stack, &mut regs);
        for r in 0..4 {
            gdb_print!(" R{} = 0x{:x}", r, regs[r + 8]);
        }
        gdb_print!("\n");
        for r in 4..8 {
            gdb_print!("    R{} = 0x{:x}", r, regs[r]);
        }
        gdb_print!("\n");
        for r in 8..12 {
            gdb_print!("    R{} = 0x{:x}", r, regs[r]);
        }
        gdb_print!("\n");
        gdb_print!(" PC = 0x{:x}\n", regs[13]);
        gdb_print!(" LR = 0x{:x}\n", regs[14]);
        gdb_print!(" xps = 0x{:x}\n", regs[15]);
    }
}
