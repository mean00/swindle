//! FreeRTOS TCB to Thread ID Mapping
//! 
//! Maintains a persistent hash map mapping raw TCB memory addresses to deterministic,
//! sequential integer Thread IDs for GDB. This ensures that even if FreeRTOS 
//! compilation options change, the debugger presents stable thread IDs.
use core::mem::MaybeUninit;
setup_log!(false);
//use crate::gdb_print;
//use crate::{bmplog, bmpwarning};
crate::gdb_print_init!();
use crate::bmp;

use heapless::LinearMap;

pub const MAX_FREERTOS_TASKS: usize = 32;

/// Global registry of known OS tasks.
/// Maps TCB address -> 1-based sequential Thread ID.
pub struct hashed_tcb {
    map: LinearMap<u32, u32, MAX_FREERTOS_TASKS>,
    index: u32,
}
// SAFETY: hashed_tcb is only used in a single-threaded debugger context.
unsafe impl Sync for hashed_tcb {}

impl hashed_tcb {
    /// Creates a new empty `hashed_tcb` starting at index 1.
    pub fn new() -> Self {
        hashed_tcb {
            map: LinearMap::new(),
            index: 1,
        }
    }
    
    /// Clears the thread map, resetting the index counter back to 1.
    pub fn clear(&mut self) {
        self.map.clear();
        self.index = 1;
    }

    /// Prune any TCB entry from the map that is no longer in the set of live tasks.
    /// This prevents address recycling bugs when tasks are deleted and created.
    pub fn prune_dead(&mut self, live_tcbs: &[u32]) {
        // Collect dead keys first (without dynamic allocation)
        let mut dead: heapless::Vec<u32, MAX_FREERTOS_TASKS> = heapless::Vec::new();
        for (tcb, _) in self.map.iter() {
            if !live_tcbs.contains(tcb) {
                let _ = dead.push(*tcb);
            }
        }
        for tcb in dead {
            self.map.remove(&tcb);
        }
    }
    
    /// Retrieves the Thread ID for a given TCB address, assigning a new sequential one if unseen.
    pub fn get(&mut self, tcb: u32) -> u32 {
        if let Some(&tid) = self.map.get(&tcb) {
            return tid;
        }

        // If map is full, evict the oldest entry to make room
        if self.map.is_full() {
            let first_key = self.map.keys().next().copied();
            if let Some(key) = first_key {
                self.map.remove(&key);
            }
        }

        let tid = self.index;
        self.index = self.index.wrapping_add(1);
        if self.index == 0 {
            self.index = 1; // Thread ID 0 is invalid in GDB protocol
        }
        let _ = self.map.insert(tcb, tid);
        tid
    }
}

//--

pub fn get_hashtcb() -> &'static mut hashed_tcb {
    static mut HASH: MaybeUninit<hashed_tcb> = MaybeUninit::uninit();
    static mut HASH_INIT: bool = false;
    unsafe {
        if !HASH_INIT {
            HASH.write(hashed_tcb::new());
            HASH_INIT = true;
        }
        HASH.assume_init_mut()
    }
}
/*
 *
 */
pub fn dump_hash_info() {
    let mut regs: [u32; 16] = [0; 16];
    let info = get_hashtcb();
    gdb_println!("Hash index = ", Hex(info.index));
    for (&val, &id) in info.map.iter() {
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
