/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011 Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2017, 2018 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of blackmagic/src/target/stm32f4.c (the F4-style FPEC flash controller),
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

//! STM32F2/F4/F7 and GD32F4 flash controller operations.
//!
//! The F4-style FPEC differs from the F1-style one in two ways that shape this
//! module: an erase names a *sector number* in `CR` instead of a page address, and
//! every program step carries its own bus width (`PSIZE`), which the user can
//! change with `mon psize`.

use crate::registers::*;
use rsbmp::gdb_println;
use rsbmp::target::{ALIGN_32BIT, CortexMAccess};

/// Program width a part starts with: 32-bit bus beats.
pub const DEFAULT_PSIZE: u8 = ALIGN_32BIT;

/// `mon psize` spellings, indexed by `PSIZE` value (align_e 0..3).
const PSIZE_NAMES: [&str; 4] = ["x8", "x16", "x32", "x64"];

/// `mon psize` spelling of `psize`.
pub fn psize_name(psize: u8) -> &'static str {
    PSIZE_NAMES[usize::from(psize) & 0x3]
}

/// Parse a `mon psize` argument; `None` for anything but x8/x16/x32/x64.
pub fn psize_from_name(name: &str) -> Option<u8> {
    PSIZE_NAMES
        .iter()
        .position(|candidate| name.eq_ignore_ascii_case(candidate))
        .map(|index| index as u8)
}

/// Unlock the flash controller; `false` leaves it locked.
pub fn unlock(cm: &mut CortexMAccess) -> bool {
    if cm.read32(flash_reg!(cr)) & F4_FPEC_CTRL_LOCK != 0 {
        cm.write32(flash_reg!(keyr), F4_FPEC_KEY1);
        cm.write32(flash_reg!(keyr), F4_FPEC_KEY2);
    }
    cm.read32(flash_reg!(cr)) & F4_FPEC_CTRL_LOCK == 0
}

/// Poll `SR` until the running operation completes; `false` on error / lost comms.
pub fn busy_wait(cm: &mut CortexMAccess) -> bool {
    let mut status = F4_FPEC_STATUS_BUSY;
    while status & F4_FPEC_STATUS_BUSY != 0 {
        status = cm.read32(flash_reg!(sr));
        if status & F4_FPEC_STATUS_ERROR_MASK != 0 || cm.check_error() {
            gdb_println!("stm32f4 flash error 0x", Hex(status));
            return false;
        }
    }
    true
}

/// Erase flash sector `sector` (the driver supplies the number, not the address).
pub fn erase_sector(cm: &mut CortexMAccess, sector: u8, psize: u8) -> bool {
    if !unlock(cm) {
        return false;
    }
    let ctrl = F4_FPEC_CTRL_EOPIE
        | F4_FPEC_CTRL_ERRIE
        | F4_FPEC_CTRL_SER
        | (u32::from(psize) << F4_FPEC_CTRL_PSIZE_SHIFT)
        | (u32::from(sector) << F4_FPEC_CTRL_SNB_SHIFT);
    cm.write32(flash_reg!(cr), ctrl);
    cm.write32(flash_reg!(cr), ctrl | F4_FPEC_CTRL_STRT);
    busy_wait(cm)
}

/// Program `data` at `dest` with `psize` wide bus beats.
pub fn program(cm: &mut CortexMAccess, dest: u32, data: &[u8], psize: u8) -> bool {
    if !unlock(cm) {
        return false;
    }
    cm.write32(
        flash_reg!(cr),
        (u32::from(psize) << F4_FPEC_CTRL_PSIZE_SHIFT) | F4_FPEC_CTRL_PG,
    );
    if !cm.write_aligned(dest, data, psize) {
        return false;
    }
    busy_wait(cm)
}

/// Mass erase: bank 1, and bank 2 as well when `bank_split` is set.
pub fn mass_erase(cm: &mut CortexMAccess, bank_split: bool) -> bool {
    if !unlock(cm) {
        return false;
    }
    let ctrl = F4_FPEC_CTRL_MER
        | if bank_split {
            F4_FPEC_CTRL_MER1
        } else {
            0
        };
    cm.write32(flash_reg!(cr), ctrl);
    cm.write32(flash_reg!(cr), ctrl | F4_FPEC_CTRL_STRT);
    busy_wait(cm)
}

// ---------------------------------------------------------------------------
// Option words (`OPTCR`, `OPTCR1`, `OPTCR2`)
// ---------------------------------------------------------------------------

/// Number of option words of `part_id` (C `stm32f4_opt_bytes_for`).
pub fn option_word_count(part_id: u16) -> usize {
    match part_id {
        // Single option word parts: F401xB/C, F410, F411.
        PART_STM32F401C | PART_STM32F410 | PART_STM32F411 => 1,
        // Three option words: F72x/F73x (has OPTCR2).
        PART_STM32F72X => 3,
        // Two option words: everyone else (F20x, F40x, F42x, F446, F401xD/E, F46x,
        // F412, F413, F74x, F76x and the GD32F4 clones).
        _ => 2,
    }
}

/// Factory defaults of the option words, and the number of valid words in the array.
pub fn option_defaults(part_id: u16) -> ([u32; 3], usize) {
    let mut val = [0u32; 3];
    let count = match part_id {
        PART_STM32F401C | PART_STM32F410 | PART_STM32F411 => {
            val[0] = 0x0fff_aaed;
            1
        }
        PART_STM32F72X => {
            val[0] = 0x0fff_aaed;
            val[1] = 0x0000_ffff;
            val[2] = 0x8000_0000;
            3
        }
        PART_STM32F74X | PART_STM32F76X => {
            val[0] = 0x0fff_aaed;
            val[1] = 0x0000_ffff;
            2
        }
        _ => {
            val[0] = 0x0fff_aaed;
            val[1] = 0x0fff_0000;
            2
        }
    };
    (val, count)
}

/// Zero out reserved bits of `val`; `false` when `val[0]` holds an illegal RDP key.
pub fn option_mask(part_id: u16, val: &mut [u32; 3]) -> bool {
    let rdp = val[0] & F4_FPEC_OPTION_CTRL_PROT_MASK;
    if rdp != F4_FPEC_OPTION_CTRL_PROT_L0 && rdp != F4_FPEC_OPTION_CTRL_PROT_L1 {
        return false;
    }
    match part_id {
        PART_STM32F401C | PART_STM32F410 | PART_STM32F411 => {
            val[0] &= 0x0fff_aaed;
        }
        PART_STM32F72X => {
            val[0] &= 0x0fff_aaed;
            val[1] &= 0x0000_ffff;
            val[2] &= 0x8000_0000;
        }
        PART_STM32F74X | PART_STM32F76X => {
            val[0] &= 0x0fff_aaed;
            val[1] &= 0x0000_ffff;
        }
        _ => {
            val[0] &= 0x0fff_aaed;
            val[1] &= 0x0fff_0000;
        }
    }
    true
}

/// Program `val[..count]` into the option words of `part_id` and reset the target.
///
/// Writing option words re-erases the whole flash, hence the seconds-long wait;
/// `false` leaves the controller as the part left it.
pub fn option_write(cm: &mut CortexMAccess, part_id: u16, val: &mut [u32; 3], count: usize) -> bool {
    val[0] &= !(F4_FPEC_OPTION_CTRL_OPTSTRT | F4_FPEC_OPTION_CTRL_OPTLOCK);

    // Lifting the read protection while the hardware watchdog fuse is set would abort
    // the option byte erase halfway, so the level 1 protection is kept in that case.
    let optcr = cm.read32(flash_reg!(optcr));
    if optcr & F4_FPEC_OPTION_CTRL_WDG_SW == 0
        && optcr & F4_FPEC_OPTION_CTRL_PROT_MASK != F4_FPEC_OPTION_CTRL_PROT_L0
        && val[0] & F4_FPEC_OPTION_CTRL_PROT_MASK != F4_FPEC_OPTION_CTRL_PROT_L1
    {
        val[0] &= !F4_FPEC_OPTION_CTRL_PROT_MASK;
        val[0] |= F4_FPEC_OPTION_CTRL_PROT_L1;
        gdb_println!("Keeping L1 protection while HW Watchdog fuse is set!");
    }

    cm.write32(flash_reg!(optkeyr), F4_FPEC_OPTION_KEY1);
    cm.write32(flash_reg!(optkeyr), F4_FPEC_OPTION_KEY2);
    if !busy_wait(cm) {
        return false;
    }

    if option_word_count(part_id) > 1 && count > 1 {
        cm.write32(flash_reg!(optcr1), val[1]);
        if part_id == PART_STM32F72X && count > 2 {
            cm.write32(flash_reg!(optcr2), val[2]);
        }
    }

    cm.write32(flash_reg!(optcr), val[0]);
    cm.write32(flash_reg!(optcr), val[0] | F4_FPEC_OPTION_CTRL_OPTSTRT);

    gdb_println!("Erasing flash");
    gdb_println!("This may take a few seconds...");
    if !busy_wait(cm) {
        return false;
    }

    cm.write32(flash_reg!(optcr), F4_FPEC_OPTION_CTRL_OPTLOCK);
    // The option bits only take effect after a reset.
    cm.reset();
    true
}

/// Read the option words of `part_id` into `val`.
pub fn read_options(cm: &CortexMAccess, part_id: u16, val: &mut [u32; 3]) {
    val[0] = cm.read32(flash_reg!(optcr));
    let words = option_word_count(part_id);
    if words > 1 {
        val[1] = cm.read32(flash_reg!(optcr1));
        if words > 2 {
            val[2] = cm.read32(flash_reg!(optcr2));
        }
    }
}
