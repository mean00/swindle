/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (c) 2021-2023 Fabrice Prost-Boucle <fabalthazar@falbalab.fr>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the flash programming routines of blackmagic/src/target/stm32g0.c
 * (`stm32g0_flash_unlock()`/`_lock()`/`_wait_busy()`/`_op_finish()`/`_flash_erase()`/
 * `_flash_write()`/`_mass_erase()`/`_option_write()`), made for the Swindle firmware.
 * This file is derived from that C source and stays under the same license as the
 * original Black Magic code reproduced in `lib.rs`.
 */

//! Flash controller of the STM32G0/C0/U0 parts.
//!
//! The G0-style FPEC differs from the F1-style one in three ways that shape this module:
//!
//! - a page erase names a *page number* (and, past page [`FLASH_BANK2_START_PAGE`], the
//!   second bank through `BKER`) instead of a page address;
//! - program steps are 64 bits wide, so the data goes through the generic memory write;
//! - once the first word of the main flash has been programmed the `ACR` `EMPTY` bit has
//!   to be cleared, otherwise the part keeps booting from its system memory.
//!
//! The OTP area is programmed through the same path, 8 bytes at a time, and erased by
//! clearing the error flags only.

use crate::registers::*;
use rsbmp::gdb_println;
use rsbmp::target::CortexMAccess;

/// Unlock the flash controller (C `stm32g0_flash_unlock()`).
pub fn unlock(cm: &CortexMAccess) {
    cm.write32(flash_reg!(keyr), FPEC_KEY1);
    cm.write32(flash_reg!(keyr), FPEC_KEY2);
}

/// Lock the flash controller again (C `stm32g0_flash_lock()`).
pub fn lock(cm: &CortexMAccess) {
    let ctrl = cm.read32(flash_reg!(cr)) | FPEC_CTRL_LOCK;
    cm.write32(flash_reg!(cr), ctrl);
}

/// Unlock the option byte block (C `stm32g0_flash_option_unlock()`).
pub fn option_unlock(cm: &CortexMAccess) {
    cm.write32(flash_reg!(optkeyr), FPEC_OPTION_KEY1);
    cm.write32(flash_reg!(optkeyr), FPEC_OPTION_KEY2);
}

/// Poll `SR` until neither bank is busy; `false` on a lost connection.
fn wait_busy(cm: &CortexMAccess) -> bool {
    while cm.read32(flash_reg!(sr)) & FPEC_STATUS_BSY_MASK != 0 {
        if cm.check_error() {
            return false;
        }
    }
    true
}

/// Clear `EOP`, drop the program bit and lock the controller (C `_flash_op_finish()`).
///
/// `CR` is written 16 bits wide on purpose: a 32-bit write of 0 would clear the reserved
/// bits of the register as well.
pub fn op_finish(cm: &CortexMAccess) {
    cm.write32(flash_reg!(sr), FPEC_STATUS_EOP);
    cm.write16(flash_reg!(cr), 0);
    lock(cm);
}

/// Clear every pending `SR` flag (`SR` is write-1-to-clear).
fn clear_status(cm: &CortexMAccess) {
    let status = cm.read32(flash_reg!(sr));
    cm.write32(flash_reg!(sr), status);
}

/// Erase the page of `region_start` holding `addr`.
///
/// In the OTP case (any address at or above [`OTP_BASE`]) this only clears the previous
/// error flags and reports success, exactly like the C driver: OTP cells cannot be
/// erased.
pub fn erase_page(cm: &CortexMAccess, region_start: u32, blocksize: usize, addr: u32) -> bool {
    if !wait_busy(cm) {
        op_finish(cm);
        return false;
    }

    clear_status(cm);

    if addr >= OTP_BASE {
        op_finish(cm);
        return true;
    }

    unlock(cm);

    // `BKER` selects the second bank, whose page numbers restart from 0.
    let page = (addr - region_start) / blocksize as u32;
    let ctrl = (page << FPEC_CTRL_PNB_SHIFT)
        | FPEC_CTRL_PER
        | if page >= FLASH_BANK2_START_PAGE {
            FPEC_CTRL_BKER
        } else {
            0
        };
    cm.write32(flash_reg!(cr), ctrl);
    cm.write32(flash_reg!(cr), ctrl | FPEC_CTRL_START);

    if !wait_busy(cm) {
        op_finish(cm);
        return false;
    }

    let status = cm.read32(flash_reg!(sr));
    if status & FPEC_STATUS_ERROR_MASK != 0 {
        gdb_println!("stm32g0 flash erase error: sr 0x", Hex(status));
    }
    op_finish(cm);
    status & FPEC_STATUS_ERROR_MASK == 0
}

/// Program `data` at `dest`, which must already be erased.
///
/// The status register is assumed ready and free of errors (the manager always erases
/// first). After the first program step of the main flash, `ACR.EMPTY` is cleared so the
/// part reboots into its flash instead of its system memory.
pub fn program(cm: &CortexMAccess, dest: u32, data: &[u8]) -> bool {
    unlock(cm);
    cm.write32(flash_reg!(cr), FPEC_CTRL_PG);
    cm.mem_write(dest, data);

    if !wait_busy(cm) {
        gdb_println!("stm32g0 flash write: comm error");
        op_finish(cm);
        return false;
    }

    let status = cm.read32(flash_reg!(sr));
    if status & FPEC_STATUS_ERROR_MASK != 0 {
        gdb_println!("stm32g0 flash write error: sr 0x", Hex(status));
        op_finish(cm);
        return false;
    }

    if dest == FLASH_BANK1_BASE && cm.read32(FLASH_BANK1_BASE) != 0xffff_ffff {
        let acr = cm.read32(flash_reg!(acr)) & !FPEC_ACCESS_CTRL_EMPTY;
        cm.write32(flash_reg!(acr), acr);
    }

    op_finish(cm);
    true
}

/// Mass erase both banks (C `stm32g0_mass_erase()`).
pub fn mass_erase(cm: &CortexMAccess) -> bool {
    erase_with(cm, FPEC_CTRL_MER1 | FPEC_CTRL_MER2 | FPEC_CTRL_START)
}

/// Mass erase one bank, `1` or `2` (C `stm32g0_cmd_erase_bank()`).
pub fn erase_bank(cm: &CortexMAccess, bank: u8) -> bool {
    let select = match bank {
        1 => FPEC_CTRL_MER1,
        2 => FPEC_CTRL_MER2,
        _ => return false,
    };
    erase_with(cm, select | FPEC_CTRL_START)
}

/// Run the erase described by `ctrl` and report whether it completed cleanly.
fn erase_with(cm: &CortexMAccess, ctrl: u32) -> bool {
    unlock(cm);
    cm.write32(flash_reg!(cr), ctrl);

    if !wait_busy(cm) {
        op_finish(cm);
        return false;
    }

    let status = cm.read32(flash_reg!(sr));
    op_finish(cm);
    status & FPEC_STATUS_ERROR_MASK == 0
}

/// Program the option registers with `values` (C `stm32g0_option_write()`).
///
/// `values` is parallel to [`OPTION_REGISTERS`]; `None` leaves a register alone, which is
/// how `mon option write` expresses "the user did not name this one" (C uses a 0 address
/// for the same thing).
///
/// The device reloads the option bytes on success, which resets it: the caller has to
/// tell the user to scan and attach again.
pub fn option_write(cm: &CortexMAccess, values: &[Option<u32>; OPTION_REGISTER_COUNT]) -> bool {
    unlock(cm);
    option_unlock(cm);

    if !wait_busy(cm) {
        op_finish(cm);
        return false;
    }

    for (index, value) in values.iter().enumerate() {
        if let Some(value) = value {
            cm.write32(OPTION_REGISTERS[index].addr, *value);
        }
    }
    cm.write32(flash_reg!(cr), FPEC_CTRL_OPTSTART);

    if !wait_busy(cm) {
        op_finish(cm);
        return false;
    }

    // `OBL_LAUNCH` makes the part load the new option bytes, which resets it.
    cm.write32(flash_reg!(cr), FPEC_CTRL_OBL_LAUNCH);
    gdb_println!("Scan and attach again");
    true
}
