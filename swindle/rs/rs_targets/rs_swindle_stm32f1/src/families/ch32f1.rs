/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2022 1BitSquared <info@1bitsquared.com>
 * Written by mean00 <fixounet@free.fr>
 *
 * Rust port of blackmagic/src/target/ch32f1.c, made for the Swindle firmware. This file
 * is derived from that C source and stays under the same license as the original Black
 * Magic code reproduced below.
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

//! WCH CH32F103 driver: an STM32F1 medium-density clone with a second, "fast" flash mode.
//!
//! The plain FPEC path is too slow on these parts, so programming goes through a 128 byte
//! write buffer: the buffer is filled at the target address, loaded with one control bit
//! and then committed by *reading* the word at `addr ^ 0x100` and writing it to the magic
//! register. The buffer only latches when no SWD traffic is in flight, which is why the
//! driver waits for the page to read back erased before touching it.
//!
//! Everything else (attach, detach, memory map, `mon` commands) comes from [`F1Driver`],
//! which is told to route erase/write here by `F1Driver::with_ch32_fast()`.
//!
//! The C driver configured no DBGMCU freeze bits (it never called
//! `stm32f1_configure_dbgmcu()`); going through [`F1Driver`] does, and the part answers at
//! the STM32F1 DBGMCU block this probe reads its id from, so the watchdogs stay frozen while
//! the core is halted just like on the parts `stm32f1.c` drove.

use super::f1::F1Driver;
use crate::registers::*;
use crate::stm32_parts::DRIVER_NAME_CH32F1;
use rsbmp::gdb_println;
use rsbmp::target::{CortexMAccess, delay_ms};

/// FPEC register that takes the two unlock keys of the fast mode (C `FLASH_MODEKEYR`).
const FPEC_MODEKEYR: u32 = FPEC_BASE + 0x24;
/// FPEC register that commits a buffered operation (C `FLASH_MAGIC`).
const FPEC_MAGIC: u32 = FPEC_BASE + 0x34;

/// Fast-mode lock bit: set while the fast mode is locked.
const FPEC_CTRL_FLOCK: u32 = 1 << 15;
/// Fast page program.
const FPEC_CTRL_FTPG: u32 = 1 << 16;
/// Fast page erase.
const FPEC_CTRL_FTER: u32 = 1 << 17;
/// Load the write buffer into the flash controller.
const FPEC_CTRL_BUF_LOAD: u32 = 1 << 18;
/// Reset the write buffer.
const FPEC_CTRL_BUF_RESET: u32 = 1 << 19;

/// Bytes the write buffer of the fast mode holds, i.e. one erase block of the part.
pub const PAGE_SIZE: usize = 128;
/// The magic word is read from the programmed address XORed with this.
const MAGIC_OFFSET: u32 = 0x100;

/// Factory flash size register, a 16 bit KiB value.
const FLASH_SIZE: u32 = 0x1fff_f7e0;
/// Size of the flash size register field.
const FLASH_SIZE_MASK: u32 = 0xffff;
/// Flash size assumed for the parts that report 0 there (CH32F103C8T6 on Blue Pill boards).
const FLASH_SIZE_FALLBACK_KIB: usize = 64;
/// SRAM size of the part.
const RAM_SIZE: usize = 0x5000;

/// Device id the CH32F103 shares with the STM32F1 medium-density parts.
const DEVICE_ID: u16 = 0x410;
/// Core revision that tells a CH32F103 apart from an STM32F1 medium-density part.
const REVISION_ID: u32 = 0x2000;
/// Shift of the core revision inside `DBGMCU_IDCODE`.
const REVISION_SHIFT: u32 = 16;
/// Value the flash reads back as when erased.
const ERASED: u32 = 0xffff_ffff;
/// How many times the driver re-reads a page while waiting for it to read back erased.
const READY_ATTEMPTS: usize = 32;
/// Pause, in milliseconds, before those reads: the controller needs quiet time to settle.
const READY_DELAY_MS: u32 = 2;

/// Probe a CH32F103 (C `ch32f1_probe()`).
pub fn probe(cm: &mut CortexMAccess) -> Option<F1Driver> {
    let cpuid_partno = cm.cpuid() & CORTEX_CPUID_PARTNO_MASK;
    if cpuid_partno != CORTEX_M3 {
        return None;
    }

    let idcode = cm.read32(F1_DBGMCU_IDCODE);
    // The device id is the STM32F1 medium-density one; only the revision and the working
    // fast-mode lock bit below tell the CH32 apart.
    if (idcode & IDCODE_DEV_ID_MASK) as u16 != DEVICE_ID || idcode >> REVISION_SHIFT != REVISION_ID {
        return None;
    }
    // Not a CH32: the fast-mode lock cannot be released on a plain STM32F1.
    if !has_fast_unlock(cm) {
        return None;
    }

    let signature = cm.read32(FLASH_SIZE) & FLASH_SIZE_MASK;
    let flash_kib = if signature == 0 {
        gdb_println!("CH32: FLASHSIZE = 0, assuming CH32F103C8T6 (64 KiB)");
        FLASH_SIZE_FALLBACK_KIB
    } else {
        signature as usize
    };

    // No target options: the part neither programs 32 bits at a time nor allows memory
    // access while the core runs (same as the C driver).
    F1Driver::new(
        DRIVER_NAME_CH32F1,
        DEVICE_ID,
        cpuid_partno,
        F1_DBGMCU_CONFIG,
        false,
        false,
        0,
    )
    .with_ram(RAM_SIZE)
    .with_flash(flash_kib * KIB, PAGE_SIZE)
    .with_ch32_fast()
    .finish(cm)
}

/// Check that the fast mode exists and can be unlocked; leaves the controller unlocked
/// (C `ch32f1_has_fast_unlock()`).
fn has_fast_unlock(cm: &mut CortexMAccess) -> bool {
    // Raise the fast lock bit, which only a CH32 can do, and wait for it to stick: the
    // controller is timing sensitive.
    ctrl_set(cm, FPEC_CTRL_FLOCK);
    delay_ms(1);
    if cm.read32(FPEC_CTRL) & FPEC_CTRL_FLOCK == 0 {
        return false;
    }

    unlock_keys(cm, FPEC_KEY);
    delay_ms(1);
    unlock_keys(cm, FPEC_MODEKEYR);
    delay_ms(1);
    cm.read32(FPEC_CTRL) & FPEC_CTRL_FLOCK == 0
}

/// Send the two unlock keys to the FPEC register `addr`.
fn unlock_keys(cm: &mut CortexMAccess, addr: u32) {
    cm.write32(addr, FPEC_KEY1);
    cm.write32(addr, FPEC_KEY2);
}

/// Enter the fast mode: the stock unlock, then the fast-mode unlock (C
/// `ch32f1_flash_unlock()`).
fn fast_unlock(cm: &mut CortexMAccess) -> bool {
    unlock_keys(cm, FPEC_KEY);
    unlock_keys(cm, FPEC_MODEKEYR);
    let ctrl = cm.read32(FPEC_CTRL);
    if ctrl & FPEC_CTRL_FLOCK != 0 {
        gdb_println!("CH32: fast unlock failed, cr: 0x", Hex(ctrl));
    }
    ctrl & FPEC_CTRL_FLOCK == 0
}

/// Lock both the stock and the fast mode again (C `ch32f1_flash_lock()`): the two lock
/// bits have to be set in the same write, otherwise `FLOCK` reads back clear.
fn fast_lock(cm: &mut CortexMAccess) -> bool {
    ctrl_set(cm, FPEC_LOCK | FPEC_CTRL_FLOCK);
    let ctrl = cm.read32(FPEC_CTRL);
    if ctrl & FPEC_CTRL_FLOCK == 0 {
        gdb_println!("CH32: fast lock failed, cr: 0x", Hex(ctrl));
    }
    ctrl & FPEC_CTRL_FLOCK != 0
}

/// Set `value` in the FPEC control register.
fn ctrl_set(cm: &mut CortexMAccess, value: u32) {
    let ctrl = cm.read32(FPEC_CTRL);
    cm.write32(FPEC_CTRL, ctrl | value);
}

/// Clear `value` in the FPEC control register.
fn ctrl_clear(cm: &mut CortexMAccess, value: u32) {
    let ctrl = cm.read32(FPEC_CTRL);
    cm.write32(FPEC_CTRL, ctrl & !value);
}

/// Poll the status register until the controller is idle.
fn busy_wait(cm: &mut CortexMAccess) -> bool {
    loop {
        let status = cm.read32(FPEC_STATUS);
        if cm.check_error() {
            return false;
        }
        if status & FPEC_STATUS_BSY == 0 {
            return true;
        }
    }
}

/// Poll the status register until the end-of-operation flag is set.
fn eop_wait(cm: &mut CortexMAccess) -> bool {
    loop {
        let status = cm.read32(FPEC_STATUS);
        if cm.check_error() {
            return false;
        }
        if status & FPEC_STATUS_EOP != 0 {
            return true;
        }
    }
}

/// Commit the buffered operation at `addr`: writing the word found at `addr ^ 0x100` into
/// the magic register is what actually starts it (C `ch32f1_write_magic()`).
fn write_magic(cm: &mut CortexMAccess, addr: u32) {
    let magic = cm.read32(addr ^ MAGIC_OFFSET);
    cm.write32(FPEC_MAGIC, magic);
}

/// Reset the write buffer (C `ch32f1_buffer_clear()`).
fn buffer_clear(cm: &mut CortexMAccess) -> bool {
    ctrl_set(cm, FPEC_CTRL_FTPG);
    ctrl_set(cm, FPEC_CTRL_BUF_RESET);
    let ok = busy_wait(cm);
    ctrl_clear(cm, FPEC_CTRL_FTPG);
    ok
}

/// Wait for the previous operation to drain and check that `addr` reads back erased.
///
/// Some CH32F103C8T6 parts on Blue Pill boards need a pause with no SWD traffic in flight,
/// hence the delay and the repeated reads (C `ch32f1_wait_flash_ready()`).
fn wait_flash_ready(cm: &mut CortexMAccess, addr: u32) -> bool {
    delay_ms(READY_DELAY_MS);
    let mut value = 0;
    for _ in 0..READY_ATTEMPTS {
        value = cm.read32(addr);
        if value == ERASED {
            break;
        }
    }
    if value != ERASED {
        gdb_println!("CH32: not erased properly at 0x", Hex(addr));
    }
    value == ERASED
}

/// Load one 16 byte group of the write buffer at `addr` (C `ch32f1_upload()`).
fn upload(cm: &mut CortexMAccess, addr: u32, group: &[u8]) -> bool {
    ctrl_set(cm, FPEC_CTRL_FTPG);
    for (index, word) in group.chunks_exact(4).enumerate() {
        cm.write32(
            addr + index as u32 * 4,
            u32::from_le_bytes([word[0], word[1], word[2], word[3]]),
        );
    }
    ctrl_set(cm, FPEC_CTRL_BUF_LOAD);
    let ok = eop_wait(cm);
    cm.write32(FPEC_STATUS, FPEC_STATUS_EOP);
    ctrl_clear(cm, FPEC_CTRL_FTPG);
    write_magic(cm, addr);
    ok
}

/// Erase the pages of `[addr, addr + len)`; the flash manager hands one block per call, but
/// the loop over `len` is kept as in the C driver.
pub(crate) fn erase(cm: &mut CortexMAccess, addr: u32, len: usize) -> bool {
    if !fast_unlock(cm) {
        gdb_println!("CH32: unlock failed");
        return false;
    }

    let mut offset = 0;
    let mut ok = true;
    while ok && offset < len {
        ctrl_set(cm, FPEC_CTRL_FTER);
        cm.write32(FPEC_ADDR, addr + offset as u32);
        ctrl_set(cm, FPEC_STRT);
        ok = eop_wait(cm);
        cm.write32(FPEC_STATUS, FPEC_STATUS_EOP);
        ctrl_clear(cm, FPEC_STRT);
        write_magic(cm, addr + offset as u32);
        offset += PAGE_SIZE;
    }

    // The controller is locked again whatever the outcome, as in the C driver.
    let status = cm.read32(FPEC_STATUS);
    fast_lock(cm);
    if status & FPEC_STATUS_ERROR_MASK != 0 {
        gdb_println!("CH32: flash erase error 0x", Hex(status));
    }
    ok && status & FPEC_STATUS_ERROR_MASK == 0
}

/// Program `data` at `dest` one 128 byte buffer at a time (C `ch32f1_flash_write()`).
pub(crate) fn write(cm: &mut CortexMAccess, dest: u32, data: &[u8]) -> bool {
    // The buffer always takes a full page, so a short tail is padded with the erased value.
    let mut page = [0xffu8; PAGE_SIZE];

    for (index, chunk) in data.chunks(PAGE_SIZE).enumerate() {
        let addr = dest + (index * PAGE_SIZE) as u32;
        if !fast_unlock(cm) {
            gdb_println!("CH32: cannot fast unlock");
            return false;
        }
        busy_wait(cm);
        // Reset the buffer, then make sure the page this buffer will be committed to is
        // gone: the controller needs the silence in between.
        buffer_clear(cm);
        if !wait_flash_ready(cm, addr) {
            return false;
        }

        page[chunk.len()..].fill(0xff);
        page[..chunk.len()].copy_from_slice(chunk);
        for step in 0..PAGE_SIZE / 16 {
            if !upload(cm, addr, &page[step * 16..]) {
                gdb_println!("CH32: cannot upload to buffer");
                return false;
            }
        }

        // Commit the buffer to flash
        ctrl_set(cm, FPEC_CTRL_FTPG);
        cm.write32(FPEC_ADDR, addr);
        ctrl_set(cm, FPEC_STRT);
        let ok = eop_wait(cm);
        cm.write32(FPEC_STATUS, FPEC_STATUS_EOP);
        ctrl_clear(cm, FPEC_CTRL_FTPG);
        write_magic(cm, addr);

        let status = cm.read32(FPEC_STATUS);
        fast_lock(cm);
        if !ok || status & FPEC_STATUS_ERROR_MASK != 0 {
            gdb_println!("CH32: flash write error 0x", Hex(status));
            return false;
        }
    }

    true
}
