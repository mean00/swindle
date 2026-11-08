//! Software breakpoint management for ARM and RISC-V targets.
//!
//! Implements software breakpoints by patching target memory with a
//! breakpoint opcode (BKPT for ARM, ebreak for RISC-V). The original
//! opcode is saved and restored when the breakpoint is removed.
//!
//! ## Two kinds, chosen by the address of the breakpoint
//!
//! - **RAM breakpoints** (`add_ram_breakpoint`/`remove_ram_breakpoint`):
//!   Patch code that lives in RAM by writing the breakpoint opcode directly.
//! - **Soft breakpoints** (`add_soft_breakpoint`/`remove_soft_breakpoint`):
//!   Patch code that lives in flash by reading the full flash page, modifying
//!   it, and overwriting the page through the flash controller (requires
//!   mass-write support).

use core::mem::MaybeUninit;

setup_log!(false);
use crate::SyncCell;
use crate::bmp;
//use crate::bmplog;

const ARM_BREAKPOINT_OPCODE: [u8; 2] = [0x00u8, 0xbeu8];
const RISCV_BREAKPOINT_OPCODE: [u8; 2] = [0x02u8, 0x90u8];

/// Ceiling on simultaneously active breakpoints of one kind.
///
/// The lists live in `.bss` (they used to be heap `Vec`s, and GDB re-inserts
/// every breakpoint on each resume, so they were touched constantly). 16
/// entries of 8 bytes cover a session's worth of `break`/`hbreak` with room to
/// spare; past that an insertion fails with an error and the patch is undone,
/// instead of growing the heap in the middle of a `continue`.
const MAX_BREAKPOINTS: usize = 16;

/// A saved breakpoint entry: address and original opcode bytes.
#[derive(Clone, Copy)]
struct PatchedBreakpoint {
    address: u32,
    old_opcode: [u8; 4],
}
/// A list of active patched breakpoints.
struct BreakpointList {
    entries: heapless::Vec<PatchedBreakpoint, MAX_BREAKPOINTS>, // address, old opcode
}
//
//
//
impl BreakpointList {
    /// Find the index of a breakpoint by address.
    fn lookup_index(&self, adr: u32) -> Option<usize> {
        self.entries.iter().position(|b| b.address == adr)
    }
    /// Check if a breakpoint at `adr` already exists.
    fn is_present(&self, adr: u32) -> bool {
        self.lookup_index(adr).is_some()
    }
    /// Get the breakpoint entry at index `dex`.
    fn at(&self, dex: usize) -> &PatchedBreakpoint {
        &self.entries[dex]
    }
}

//
//
//
static RAM_BREAKPOINTS: SyncCell<BreakpointList> = SyncCell::new(BreakpointList {
    entries: heapless::Vec::new(),
});
static SOFT_BREAKPOINTS: SyncCell<BreakpointList> = SyncCell::new(BreakpointList {
    entries: heapless::Vec::new(),
});
const MAX_PAGE_SIZE: u32 = 256; // must be a power of 2!
static PAGE_BUFFER: SyncCell<MaybeUninit<[u8; MAX_PAGE_SIZE as usize]>> =
    SyncCell::new(MaybeUninit::uninit());

fn page_buffer(size: u32) -> &'static mut [u8] {
    if size > MAX_PAGE_SIZE {
        panic!("flash page too big");
    }
    unsafe { &mut (*PAGE_BUFFER.as_ptr()).assume_init_mut()[0..(size as usize)] }
}

/// Page size the target's flash-patch helper works on, or `None` when it has none we can
/// use.
///
/// The buffer is fixed and the address arithmetic assumes a power of two, so a helper
/// reporting anything else is refused instead of trusted (and never panics a `continue`).
fn patch_page_size() -> Option<u32> {
    let page_size = bmp::bmp_get_mw_page_size();
    (page_size != 0 && page_size.is_power_of_two() && page_size <= MAX_PAGE_SIZE).then_some(page_size)
}

//
//
//
fn ram_breakpoints() -> &'static mut BreakpointList {
    unsafe { RAM_BREAKPOINTS.as_mut() }
}
fn soft_breakpoints() -> &'static mut BreakpointList {
    unsafe { SOFT_BREAKPOINTS.as_mut() }
}
/// Forget all RAM breakpoints (the list only, target memory is untouched).
pub fn clear_ram_breakpoints() {
    ram_breakpoints().entries.clear();
}
/// Forget all soft breakpoints (the list only, target flash is untouched).
pub fn clear_soft_breakpoints() {
    soft_breakpoints().entries.clear();
}
/// Forget all RAM and soft breakpoints.
pub fn clear_all_breakpoints() {
    clear_ram_breakpoints();
    clear_soft_breakpoints();
}
/// Patch a breakpoint opcode into a code buffer at the given offset.
///
/// Uses BKPT (0xBE00) for ARM or ebreak (0x9002) for RISC-V.
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

/// Add a RAM breakpoint: patch the opcode at `address` with a plain memory write.
///
/// Reads the current opcode at `address`, saves it, and writes the
/// breakpoint opcode. Returns `true` on success.
pub fn add_ram_breakpoint(address: u32, _len: u32) -> bool {
    if ram_breakpoints().is_present(address) {
        // already done
        return true;
    }
    // Read old opcode
    let mut breakpoint = PatchedBreakpoint {
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
    if ram_breakpoints().entries.push(breakpoint).is_err() {
        // Table full: undo the patch, otherwise a breakpoint would be planted
        // in the target that this list cannot remove again.
        bmplog!("sw breakpoint table full ({})\n", MAX_BREAKPOINTS);
        bmp::bmp_mem_write(aligned_address, &breakpoint.old_opcode);
        return false;
    }
    true
}
/// Remove a RAM breakpoint.
///
/// Restores the original opcode that was saved when the breakpoint was added.
pub fn remove_ram_breakpoint(address: u32, _len: u32) -> bool {
    loop {
        let r = ram_breakpoints().lookup_index(address);
        match r {
            None => return true, // all done
            Some(index) => {
                // put back the old code
                let aligned_address: u32 = address & 0xfffffffcu32;
                if !bmp::bmp_mem_write(aligned_address, &ram_breakpoints().at(index).old_opcode) {
                    bmplog!("cant restore old sw value\n");
                }
                // remove X from list
                ram_breakpoints().entries.remove(index);
            }
        };
    }
}

/// Add a soft breakpoint: patch the flash page holding `address`.
///
/// Reads the full flash page containing `address`, patches the breakpoint
/// opcode, and overwrites the page. Returns `true` on success.
pub fn add_soft_breakpoint(address: u32) -> bool {
    if soft_breakpoints().is_present(address) {
        /*
         * The address is remembered as patched, but only trust that entry if
         * the target really still holds the breakpoint opcode: the flash can be
         * rewritten behind our back (GDB `load`ing a new image, or another
         * agent flashing the part) while this list keeps the old entry.
         * Reporting success then claims a breakpoint that is not in the
         * target's memory -- GDB believes it planted one, no ebreak ever
         * executes, and the breakpoint silently never fires. Re-reading is
         * cheap, so verify, and drop stale entries to re-patch below.
         */
        let aligned_address: u32 = address & 0xfffffffcu32;
        let offset_u: usize = (address & 2) as usize;
        let expected: [u8; 2] = if bmp::bmp_is_riscv() {
            RISCV_BREAKPOINT_OPCODE
        } else {
            ARM_BREAKPOINT_OPCODE
        };
        let mut current: [u8; 4] = [0, 0, 0, 0];
        if bmp::bmp_read_mem(aligned_address, &mut current)
            && current[offset_u] == expected[0]
            && current[offset_u + 1] == expected[1]
        {
            return true; // still patched, the entry is accurate
        }
        if let Some(index) = soft_breakpoints().lookup_index(address) {
            soft_breakpoints().entries.remove(index);
        }
    }
    // Read old opcode
    let mut breakpoint = PatchedBreakpoint {
        address,
        old_opcode: [0, 0, 0, 0],
    };
    // store breakpoint
    let Some(page_size) = patch_page_size() else {
        bmplog!("no usable flash page helper\n");
        return false;
    };
    let aligned_address: u32 = address & !(page_size - 1);
    let offset: u32 = address - aligned_address;
    let offset_u: usize = offset as usize;
    let bf = page_buffer(page_size);
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
    if soft_breakpoints().entries.push(breakpoint).is_err() {
        // Table full: restore the page we just patched before giving up.
        bmplog!("flash breakpoint table full ({})\n", MAX_BREAKPOINTS);
        bf[offset_u] = breakpoint.old_opcode[0];
        bf[offset_u + 1] = breakpoint.old_opcode[1];
        bmp::bmp_overwrite_flash(aligned_address, bf);
        return false;
    }
    true
}
/// Remove a soft breakpoint: restore the flash page holding `address`.
///
/// Reads the flash page, restores the original opcode, and overwrites
/// the page.
pub fn remove_soft_breakpoint(address: u32) -> bool {
    let index = match soft_breakpoints().lookup_index(address) {
        None => return true, // all done
        Some(index) => index,
    };
    // put back the old code
    let Some(page_size) = patch_page_size() else {
        bmplog!("no usable flash page helper\n");
        return false;
    };
    let aligned_address: u32 = address & !(page_size - 1);
    let offset = address - aligned_address;
    let offset_u: usize = offset as usize;
    let bf = page_buffer(page_size);
    if !(bmp::bmp_read_mem(aligned_address, bf)) {
        bmplog!("cant read old mw value\n");
        return false;
    }
    let bk = soft_breakpoints().at(index);

    bf[offset_u] = bk.old_opcode[0];
    bf[offset_u + 1] = bk.old_opcode[1];
    if !bmp::bmp_overwrite_flash(aligned_address, bf) {
        bmplog!("cant restore old sw value\n");
    }
    // remove X from list
    soft_breakpoints().entries.remove(index);
    true
}

// EOF
