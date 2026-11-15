/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of blackmagic/src/target/stm32f1.c (the F1-style FPEC flash controller),
 * made for the Swindle firmware. This file is derived from that C source and stays
 * under the same license as the original Black Magic code reproduced below.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

//! STM32F1-style FPEC flash controller operations.
//!
//! One implementation serves every family this crate drives: bank 1 lives at FPEC
//! offset 0, bank 2 at [`FPEC_BANK2_OFFSET`], and the only per-part variation is
//! whether the part has a second bank and whether a program step may write 32 bits
//! (GD32/AT32) or only 16 (STM32).

use crate::registers::*;
use crate::stm32_parts::PART_GD32_MEDIUM;
use rsbmp::gdb_println;
use rsbmp::target::{ALIGN_16BIT, ALIGN_32BIT, CortexMAccess};

/// How the FPEC instance of a part behaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FpecLayout {
    /// The part has a second flash bank (register offset [`FPEC_BANK2_OFFSET`]).
    pub dual_bank: bool,
    /// Program steps may write 32 bits at a time instead of 16.
    pub wide_writes: bool,
}

impl FpecLayout {
    /// Bus beat width for flash programming on this part.
    pub fn align(&self) -> u8 {
        if self.wide_writes { ALIGN_32BIT } else { ALIGN_16BIT }
    }

    /// FPEC register offset of the bank holding `addr`.
    pub fn bank_offset_for(&self, addr: u32) -> u32 {
        if addr >= FLASH_BANK_SPLIT {
            FPEC_BANK2_OFFSET
        } else {
            FPEC_BANK1_OFFSET
        }
    }

    /// How many bytes of `[addr, addr + len)` fall into bank 1.
    pub fn bank1_length(&self, addr: u32, len: usize) -> usize {
        if addr >= FLASH_BANK_SPLIT {
            return 0;
        }
        let end = addr as usize + len;
        if end > FLASH_BANK_SPLIT as usize {
            FLASH_BANK_SPLIT as usize - addr as usize
        } else {
            len
        }
    }
}

/// How many bytes of flash the manager hands [`write`] per call: its write chunk.
pub const WRITE_CHUNK: usize = 1024;

/// Unlock the FPEC of `bank_offset`; `false` leaves the controller locked.
pub fn unlock(cm: &mut CortexMAccess, bank_offset: u32) -> bool {
    cm.write32(flash_reg!(bank_offset, keyr), FPEC_KEY1);
    cm.write32(flash_reg!(bank_offset, keyr), FPEC_KEY2);
    let ctrl = cm.read32(flash_reg!(bank_offset, cr));
    if ctrl & FPEC_LOCK != 0 {
        gdb_println!("unlock failed, cr: 0x", Hex(ctrl));
    }
    ctrl & FPEC_LOCK == 0
}

/// Clear the end-of-operation flag (`EOP` is write-1-to-clear).
fn clear_eop(cm: &mut CortexMAccess, bank_offset: u32) {
    let status = cm.read32(flash_reg!(bank_offset, sr));
    cm.write32(flash_reg!(bank_offset, sr), status | FPEC_STATUS_EOP);
}

/// Poll `FLASH_SR` until the running operation completes; `false` on error / lost comms.
fn busy_wait(cm: &mut CortexMAccess, bank_offset: u32) -> bool {
    // Checking EOP is only legal because every operation is preceded by `clear_eop()`;
    // without it the flag could be stale from the previous operation.
    let mut status = FPEC_STATUS_BSY;
    while status & FPEC_STATUS_EOP == 0 && status & FPEC_STATUS_BSY != 0 {
        status = cm.read32(flash_reg!(bank_offset, sr));
        if cm.check_error() {
            return false;
        }
    }
    if status & FPEC_STATUS_ERROR_MASK != 0 {
        gdb_println!("stm32f1 flash error 0x", Hex(status));
    }
    status & FPEC_STATUS_ERROR_MASK == 0
}

/// Erase the page of `addr` (one call per page, like the C driver).
pub fn erase_page(cm: &mut CortexMAccess, layout: &FpecLayout, addr: u32) -> bool {
    let bank_offset = layout.bank_offset_for(addr);
    if !unlock(cm, bank_offset) {
        return false;
    }
    clear_eop(cm, bank_offset);

    cm.write32(flash_reg!(bank_offset, cr), FPEC_PER);
    cm.write32(flash_reg!(bank_offset, ar), addr);
    cm.write32(flash_reg!(bank_offset, cr), FPEC_STRT | FPEC_PER);
    busy_wait(cm, bank_offset)
}


/// Program `data` at `dest`, splitting the transfer at the bank boundary.
pub fn write(cm: &mut CortexMAccess, layout: &FpecLayout, dest: u32, data: &[u8]) -> bool {
    let align = layout.align();
    let bank1_len = layout.bank1_length(dest, data.len());

    if bank1_len != 0 {
        clear_eop(cm, FPEC_BANK1_OFFSET);
        cm.write32(flash_reg!(cr), FPEC_PG);
        if !cm.write_aligned(dest, &data[..bank1_len], align) || !busy_wait(cm, FPEC_BANK1_OFFSET) {
            return false;
        }
    }

    let remainder = data.len() - bank1_len;
    if layout.dual_bank && remainder != 0 {
        clear_eop(cm, FPEC_BANK2_OFFSET);
        cm.write32(flash_reg!(FPEC_BANK2_OFFSET, cr), FPEC_PG);
        let bank2_dest = dest + bank1_len as u32;
        if !cm.write_aligned(bank2_dest, &data[bank1_len..], align) ||
            !busy_wait(cm, FPEC_BANK2_OFFSET)
        {
            return false;
        }
    }

    true
}

/// Mass erase one bank.
fn mass_erase_bank(cm: &mut CortexMAccess, bank_offset: u32) -> bool {
    if !unlock(cm, bank_offset) {
        return false;
    }
    clear_eop(cm, bank_offset);

    cm.write32(flash_reg!(bank_offset, cr), FPEC_MER);
    cm.write32(flash_reg!(bank_offset, cr), FPEC_STRT | FPEC_MER);
    busy_wait(cm, bank_offset)
}

/// Mass erase every bank of the part.
pub fn mass_erase(cm: &mut CortexMAccess, layout: &FpecLayout) -> bool {
    if !unlock(cm, FPEC_BANK1_OFFSET) {
        return false;
    }
    if !mass_erase_bank(cm, FPEC_BANK1_OFFSET) {
        return false;
    }
    if layout.dual_bank {
        return mass_erase_bank(cm, FPEC_BANK2_OFFSET);
    }
    true
}

// ---------------------------------------------------------------------------
// Option bytes
// ---------------------------------------------------------------------------

/// Erase the whole option byte block.
pub fn option_erase(cm: &mut CortexMAccess) -> bool {
    clear_eop(cm, FPEC_BANK1_OFFSET);

    cm.write32(flash_reg!(cr), FPEC_OPTER | FPEC_OPTWRE);
    cm.write32(flash_reg!(cr), FPEC_STRT | FPEC_OPTER | FPEC_OPTWRE);
    busy_wait(cm, FPEC_BANK1_OFFSET)
}

/// Program option byte `offset` with `value`, skipping erased halfwords.
///
/// `write16_broken` selects the 32-bit write workaround needed by parts whose
/// `target_mem32_write16()` does not latch (GD32E230).
pub fn option_write_erased(
    cm: &mut CortexMAccess,
    offset: u16,
    value: u16,
    write16_broken: bool,
) -> bool {
    if value == 0xffff {
        return true;
    }

    clear_eop(cm, FPEC_BANK1_OFFSET);
    cm.write32(flash_reg!(cr), FPEC_OPTPG | FPEC_OPTWRE);

    let addr = FPEC_OBP_RDP + u32::from(offset) * 2;
    if write16_broken {
        cm.write32(addr, 0xffff_0000 | u32::from(value));
    } else {
        cm.write16(addr, value);
    }

    let result = busy_wait(cm, FPEC_BANK1_OFFSET);
    if result || offset != 0 {
        return result;
    }
    // Writing option byte 0 (RDP) fails with PGERR when the part refuses to lift the
    // read protection; treat that as "the user has to erase the option bytes".
    let status = cm.read32(flash_reg!(sr)) & FPEC_STATUS_ERROR_MASK;
    status == FPEC_STATUS_PROG_ERROR
}

/// Value that leaves the flash readable on `part_id`.
pub fn readable_key(part_id: u16) -> u16 {
    match part_id {
        // STM32F30x / F37x / F303x6/8 / F0 family: inverted key.
        0x422 | 0x432 | 0x438 | 0x440 | 0x445 | 0x446 | 0x448 | 0x442 => FPEC_OBP_RDP_KEY_F3,
        _ => FPEC_OBP_RDP_KEY,
    }
}

/// Read the whole option byte block (8 halfwords) into `out`.
pub fn read_option_bytes(cm: &mut CortexMAccess, out: &mut [u16; 8]) {
    for (index, halfword) in out.iter_mut().enumerate() {
        *halfword = cm.read16(FPEC_OBP_RDP + index as u32 * 2);
    }
}

/// Program `value` into the option byte at `addr`, rewriting the block only if needed.
pub fn option_write(cm: &mut CortexMAccess, part_id: u16, addr: u32, value: u16) -> bool {
    let index = (addr.wrapping_sub(FPEC_OBP_RDP) >> 1) as u16;
    // A negative index wraps into a huge positive number, hence the cast above.
    if u32::from(index) > u32::from(STM32F1_OB_COUNT) - 1 {
        return false;
    }

    let mut opt_val = [0u16; STM32F1_OB_COUNT as usize];
    read_option_bytes(cm, &mut opt_val);

    if opt_val[index as usize] == value {
        return true;
    }
    if opt_val[index as usize] != 0xffff && !option_erase(cm) {
        return false;
    }
    opt_val[index as usize] = value;

    // GD32E230 is the one part whose 16-bit write does not latch.
    let write16_broken = part_id == PART_GD32_MEDIUM;
    for (offset, halfword) in opt_val.iter().enumerate() {
        if !option_write_erased(cm, offset as u16, *halfword, write16_broken) {
            return false;
        }
    }
    true
}
