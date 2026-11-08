/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the STM32F0/F1/F3 part table, memory map and flash programming
 * routines of blackmagic/src/target/stm32f1.c, made for the Swindle firmware. This
 * file is derived from that C source and stays under the same license as the original
 * Black Magic code reproduced below.
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

//! STM32F1/F0/F3 and GD32F1/F2/F3/E230/E5 family driver.
//!
//! These parts share one flash controller (FPEC) and one attach prologue (the DBGMCU
//! freeze bits), so a single driver covers them; what the probe picks out is the part
//! name, the memory map and the two FPEC quirks (second bank, 32-bit program steps).

use crate::flash::{self, FpecLayout};
use crate::registers::*;
use crate::stm32_parts::*;
use super::ch32f1;
use rs_stm32_common::Stm32Driver;
use rsbmp::target::{
    CortexMAccess, MemoryMap, PartDescriptor, RustArmTargetDriver, TOPT_NON_HALTING_MEM_IO,
};

/// Options the C driver used to flag parts that accept 32-bit program steps; the Rust
/// driver keeps the flag in [`F1Driver::layout`] instead, but it also has to reach the
/// C `target_s` because `mon option`/`mon erase_bank` inspect it.
pub const TOPT_32BIT_WRITES: u32 = 1 << 8;

/// Everything the `mon` commands of this family need to know about the part.
///
/// A view built on demand from the live driver by [`active_info()`]; the probe results
/// themselves are only stored once, in [`F1Driver::info`].
#[derive(Debug, Clone, Copy)]
pub struct F1Info {
    pub part_id: u16,
    pub cpuid_partno: u32,
    pub wide_writes: bool,
}

/// State of the part the driver is attached to, or `None` when another driver is.
///
/// The `mon` callbacks get no driver handle (the C trampoline passes only the packet text),
/// so they resolve the live driver in the ARM dispatcher and read its own `info`.
pub fn active_info() -> Option<F1Info> {
    rsbmp::arm::driver_registry::active_arm_driver_as::<F1Driver>().map(|driver| driver.info)
}

/// The driver instance for one probed part.
///
/// The memory map is kept as compact numbers instead of `FlashDesc`/`RamDesc` arrays:
/// the swindle boards are RAM-tight and the regions are rebuilt from them by
/// [`Self::memory_map`] only while the shim creates the C nodes.
#[derive(Debug, Clone)]
pub struct F1Driver {
    /// Debug-side core shared with the other STM32 families (name, part id, freeze bits).
    common: Stm32Driver,
    info: F1Info,
    layout: FpecLayout,
    /// Bank 1: length and block (page) size, in bytes.
    bank1_len: usize,
    bank1_block: usize,
    /// Bank 2 length, 0 when the part has a single bank.
    bank2_len: usize,
    bank2_block: usize,
    /// Size of the main SRAM region at [`SRAM_BASE`].
    ram_len: usize,
    /// One more RAM region at an address of the part's choosing, e.g. the STM32F3
    /// core-coupled RAM or the second SRAM bank of the MM32F52.
    extra_ram: Option<(u32, usize)>,
    /// The part has the CRC unit the hardware CRC stub streams through (GD32 clones).
    hw_crc32: bool,
    /// The part programs through the CH32 fast-mode write buffer (see [`ch32f1`]).
    ch32_fast: bool,
}

/// Base address and size of the STM32F3 core-coupled RAM.
const CCM_RAM_BASE: u32 = 0x1000_0000;
const CCM_RAM_LEN: usize = 0x4000;

impl F1Driver {
    /// Build a driver from the part description the probe matched.
    ///
    /// Kept out of line so the four probes that call it share one copy of the builder.
    #[inline(never)]
    pub(crate) fn new(
        name: &'static str,
        part_id: u16,
        cpuid_partno: u32,
        dbgmcu_config: u32,
        wide_writes: bool,
        dual_bank: bool,
        options: u32,
    ) -> Self {
        // The F0 parts live behind their own DBGMCU block, whose freeze bits sit in a
        // separate APB1 register; the F1/F3 parts keep both halves in `DBGMCU_CR`.
        let common = if dbgmcu_config == F0_DBGMCU_CONFIG {
            Stm32Driver::new(name, u32::from(part_id), options)
                .with_freeze(
                    F0_DBGMCU_APB1FREEZE,
                    F0_DBGMCU_APB1FREEZE_IWDG | F0_DBGMCU_APB1FREEZE_WWDG,
                )
                .with_freeze(
                    F0_DBGMCU_CONFIG,
                    F0_DBGMCU_CONFIG_DBG_STANDBY | F0_DBGMCU_CONFIG_DBG_STOP,
                )
        } else {
            Stm32Driver::new(name, u32::from(part_id), options).with_freeze(
                dbgmcu_config,
                F1_DBGMCU_CONFIG_WWDG_STOP
                    | F1_DBGMCU_CONFIG_IWDG_STOP
                    | F1_DBGMCU_CONFIG_DBG_STANDBY
                    | F1_DBGMCU_CONFIG_DBG_STOP
                    | F1_DBGMCU_CONFIG_DBG_SLEEP,
            )
        };

        Self {
            common,
            info: F1Info { part_id, cpuid_partno, wide_writes },
            layout: FpecLayout { dual_bank, wide_writes },
            bank1_len: 0,
            bank1_block: 0,
            bank2_len: 0,
            bank2_block: 0,
            ram_len: 0,
            extra_ram: None,
            hw_crc32: false,
            ch32_fast: false,
        }
    }

    /// Set the bank 1 flash geometry (start is always [`FLASH_BANK1_BASE`]).
    pub(crate) fn with_flash(mut self, length: usize, blocksize: usize) -> Self {
        self.bank1_len = length;
        self.bank1_block = blocksize;
        self
    }

    /// Add the second flash bank (start is always [`FLASH_BANK2_BASE`]).
    pub(crate) fn with_bank2(mut self, length: usize, blocksize: usize) -> Self {
        self.bank2_len = length;
        self.bank2_block = blocksize;
        self
    }

    /// Set the size of the main SRAM region at [`SRAM_BASE`].
    pub(crate) fn with_ram(mut self, length: usize) -> Self {
        self.ram_len = length;
        self
    }

    /// Also expose the RAM region at `start` (the STM32F3 CCM RAM, the MM32F52 second
    /// SRAM bank).
    pub(crate) fn with_extra_ram(mut self, start: u32, length: usize) -> Self {
        self.extra_ram = Some((start, length));
        self
    }

    /// The part can checksum a range through its own CRC unit (GD32 clones).
    pub(crate) fn with_hw_crc32(mut self) -> Self {
        self.hw_crc32 = true;
        self
    }

    /// Program through the CH32 fast-mode write buffer instead of the stock FPEC path.
    pub(crate) fn with_ch32_fast(mut self) -> Self {
        self.ch32_fast = true;
        self
    }

    /// Finish the probe-time setup; the `mon` commands read the part through [`active_info()`].
    pub(crate) fn finish(self, cm: &mut CortexMAccess) -> Option<Self> {
        self.common.configure(cm);
        Some(self)
    }

    /// FPEC layout of this part.
    pub fn layout(&self) -> FpecLayout {
        self.layout
    }
}


/// Read the part's DBGMCU `IDCODE`, together with the address of the matching DBGMCU
/// control register (the STM32F0 and GD32E5 parts hide it behind a different block).
fn read_idcode(cm: &mut CortexMAccess) -> (u16, u32) {
    let (idcode, config) = match cm.cpuid() & CORTEX_CPUID_PARTNO_MASK {
        CORTEX_M0 | CORTEX_M23 => (F0_DBGMCU_IDCODE, F0_DBGMCU_CONFIG),
        CORTEX_M33 => (GD32E5_DBGMCU_IDCODE, GD32E5_DBGMCU_CONFIG),
        _ => (F1_DBGMCU_IDCODE, F1_DBGMCU_CONFIG),
    };
    ((cm.read32(idcode) & IDCODE_DEV_ID_MASK) as u16, config)
}

/// Probe the STM32F1/F0/F3 parts (C `stm32f1_probe`).
pub fn probe_stm32(cm: &mut CortexMAccess) -> Option<F1Driver> {
    let (part_id, dbgmcu_config) = read_idcode(cm);
    let cpuid_partno = cm.cpuid() & CORTEX_CPUID_PARTNO_MASK;
    // Every STM32 part in this family allows SRAM access without halting the core.
    let options = TOPT_NON_HALTING_MEM_IO;

    // Every part here programs 16 bits at a time, and only the XL density parts have a
    // second bank, so one constructor covers all but that one.
    let single_bank = |name: &'static str, ram: usize, flash: usize, page: usize| {
        F1Driver::new(name, part_id, cpuid_partno, dbgmcu_config, false, false, options)
            .with_ram(ram)
            .with_flash(flash, page)
    };

    let driver = match part_id {
        // CS clone / medium / low / value line
        PART_CS_CLONE | PART_STM32F1_MEDIUM | PART_STM32F1_LOW | PART_STM32F1_VALUE_LM => {
            // Clone parts report a core revision above 1 in the AP IDR.
            let name = if cm.ap_idr() >> AP_IDR_REVISION_SHIFT >= F1_MIN_CLONE_CORE_REVISION {
                DRIVER_NAME_F1_CLONE
            } else {
                DRIVER_NAME_F1_LM
            };
            single_bank(name, F1_LM_DENSITY_RAM, F1_LM_DENSITY_FLASH, F1_LM_DENSITY_PAGE)
        }
        // High density / connectivity line / value line high density
        PART_STM32F1_HIGH | PART_STM32F1_CONNECTIVITY | PART_STM32F1_VALUE_HD => single_bank(
            DRIVER_NAME_F1_VL,
            F1_HD_DENSITY_RAM,
            F1_HD_DENSITY_FLASH,
            F1_HD_DENSITY_PAGE,
        ),
        // XL density: the second bank is a separate flash region
        PART_STM32F1_XL => F1Driver::new(
            DRIVER_NAME_F1_XL,
            part_id,
            cpuid_partno,
            dbgmcu_config,
            false,
            true,
            options,
        )
        .with_ram(F1_XL_DENSITY_RAM)
        .with_flash(F1_HD_DENSITY_FLASH, F1_HD_DENSITY_PAGE)
        .with_bank2(F1_HD_DENSITY_FLASH, F1_HD_DENSITY_PAGE),
        // STM32F303x6/8, STM32F328, STM32F30x, STM32F303xD/E, STM32F398xE: also have CCM RAM
        PART_STM32F303X6X8 | PART_STM32F30X | PART_STM32F303XDE => {
            single_bank(DRIVER_NAME_F3, F3_RAM, F3_FLASH, F3_PAGE)
                .with_extra_ram(CCM_RAM_BASE, CCM_RAM_LEN)
        }
        // STM32F37x, STM32F302C8
        PART_STM32F37X | PART_STM32F302C8 => single_bank(DRIVER_NAME_F3, F3_RAM, F3_FLASH, F3_PAGE),
        PART_STM32F03 => single_bank(DRIVER_NAME_F03, F03_RAM, F03_FLASH, F03_PAGE),
        PART_STM32F04 => single_bank(DRIVER_NAME_F04, F04_RAM, F04_FLASH, F04_PAGE),
        PART_STM32F05 => single_bank(DRIVER_NAME_F05, F05_RAM, F05_FLASH, F05_PAGE),
        PART_STM32F07 => single_bank(DRIVER_NAME_F07, F07_RAM, F07_FLASH, F07_PAGE),
        PART_STM32F09 => single_bank(DRIVER_NAME_F09, F09_RAM, F09_FLASH, F09_PAGE),
        _ => return None,
    };

    driver.finish(cm)
}

/// Probe the GD32F1/F2/F3/E230/E5 parts (C `gd32f1_probe`).
pub fn probe_gd32(cm: &mut CortexMAccess) -> Option<F1Driver> {
    let (part_id, dbgmcu_config) = read_idcode(cm);
    let cpuid_partno = cm.cpuid() & CORTEX_CPUID_PARTNO_MASK;
    // Gigadevices program 32 bits at a time.
    let wide = true;

    let (name, page, non_halting) = match part_id {
        // GD32F30x_HD / GD32F30x_XD
        PART_GD32F30X_HD | PART_GD32F30X_XD => (DRIVER_NAME_GD32F3_HDXD, GD32_HD_PAGE, true),
        // Connectivity line
        PART_GD32F2 => (DRIVER_NAME_GD32F2, GD32_HD_PAGE, false),
        // Medium density: the driver name tells the three cores apart
        PART_GD32_MEDIUM => match cpuid_partno {
            CORTEX_M23 => (DRIVER_NAME_GD32E230, GD32_MEDIUM_PAGE, false),
            CORTEX_M4 => (DRIVER_NAME_GD32F3_MD, GD32_HD_PAGE, false),
            _ => (DRIVER_NAME_GD32F1, GD32_MEDIUM_PAGE, true),
        },
        // GD32E50x_CL
        PART_GD32E5 => (DRIVER_NAME_GD32E5, GD32E5_PAGE, true),
        _ => return None,
    };

    let signature = cm.read32(GD32FX_FLASHSIZE);
    let flash_kib = (signature & GD32_FLASH_KIB_MASK) as usize;
    let ram_kib = (signature >> GD32_RAM_KIB_SHIFT) as usize;
    let options = if non_halting { TOPT_NON_HALTING_MEM_IO } else { 0 } | TOPT_32BIT_WRITES;

    let dual_bank = flash_kib > GD32_DUAL_BANK_MIN_KIB;
    let mut driver = F1Driver::new(
        name,
        part_id,
        cpuid_partno,
        dbgmcu_config,
        wide,
        dual_bank,
        options,
    )
    .with_ram(ram_kib * KIB)
    .with_hw_crc32();

    if dual_bank {
        // Bank 1 holds the first part of the flash in `page` pages, bank 2 the rest in
        // the smaller pages of the extra density parts.
        driver = driver
            .with_flash(GD32_DUAL_BANK_MIN_KIB * KIB, page)
            .with_bank2((flash_kib - GD32_DUAL_BANK_MIN_KIB) * KIB, GD32_BANK2_PAGE);
    } else {
        driver = driver.with_flash(flash_kib * KIB, page);
    }

    driver.finish(cm)
}

/// The shared 256-byte driver cell must hold this driver
/// (`rsbmp::target::DRIVER_STORAGE_SIZE`): a build error here is better than the runtime
/// check `install()` would make at the first probe.
const _: () = assert!(core::mem::size_of::<F1Driver>() <= rsbmp::target::DRIVER_STORAGE_SIZE);

impl PartDescriptor for F1Driver {
    fn name(&self) -> &'static str {
        self.common.name()
    }

    fn part_id(&self) -> u32 {
        self.common.part_id()
    }

    fn target_options(&self) -> u32 {
        self.common.options()
    }

    fn memory_map(&self) -> MemoryMap {
        // A part whose flash the probe could not size (a read protected AT32) gets no flash
        // region at all, only its RAM.
        let mut map = MemoryMap::new();
        if self.bank1_len != 0 {
            map = map.with_flash(
                FLASH_BANK1_BASE,
                self.bank1_len,
                self.bank1_block,
                flash::WRITE_CHUNK,
            );
        }
        if self.bank2_len != 0 {
            map = map.with_flash(
                FLASH_BANK2_BASE,
                self.bank2_len,
                self.bank2_block,
                flash::WRITE_CHUNK,
            );
        }
        if let Some((start, length)) = self.extra_ram {
            map = map.with_ram(start, length);
        }
        map.with_ram(SRAM_BASE, self.ram_len)
    }

    fn commands(&self) -> &'static [rsbmp::commands::CommandTree] {
        &crate::mon_commands::COMMANDS
    }
}

impl RustArmTargetDriver for F1Driver {
    fn flash_erase(&mut self, cm: &mut CortexMAccess, addr: u32, len: usize) -> bool {
        if self.ch32_fast {
            return ch32f1::erase(cm, addr, len);
        }
        // The flash manager calls this once per block, so only the page holding `addr`
        // has to be erased (same contract as the C driver).
        flash::erase_page(cm, &self.layout, addr)
    }

    fn flash_write(&mut self, cm: &mut CortexMAccess, dest: u32, data: &[u8]) -> bool {
        if self.ch32_fast {
            return ch32f1::write(cm, dest, data);
        }
        flash::write(cm, &self.layout, dest, data)
    }

    fn mass_erase(&mut self, cm: &mut CortexMAccess) -> bool {
        flash::mass_erase(cm, &self.layout)
    }

    fn crc32(&mut self, cm: &mut CortexMAccess, addr: u32, len: u32) -> Option<u32> {
        // Only the GD32 clones have the CRC unit; on the STM32 parts the range goes
        // through the generic debug-link CRC (same as the C drivers).
        if self.hw_crc32 {
            crate::crc32::gd32_crc32(cm, addr, len as usize)
        } else {
            None
        }
    }

    fn attach(&mut self, cm: &mut CortexMAccess) -> bool {
        // Attach first, then (re)apply the freeze bits: detach undid them.
        self.common.attach(cm)
    }

    fn detach(&mut self, cm: &mut CortexMAccess) {
        self.common.detach(cm);
    }
}


