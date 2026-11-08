/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the Artery AT32F40x and MindMotion MM32 probes of
 * blackmagic/src/target/stm32f1.c, made for the Swindle firmware. This file is derived
 * from that C source and stays under the same license as the original Black Magic code
 * reproduced below.
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

//! Probes of the STM32F1-style clones that are neither STM32 nor GD32: the Artery AT32
//! and the MindMotion MM32 lines.
//!
//! All of them run the same FPEC flash controller as the F1 family ([`crate::flash`]), so
//! the probe only has to find the part in its table; what a table row carries is the
//! geometry (flash and SRAM size, page size, a second bank), the `DBGMCU` block the freeze
//! bits live in and whether the part programs 32 bits at a time.

use crate::families::f1::{F1Driver, TOPT_32BIT_WRITES};
use crate::registers::*;
use crate::stm32_parts::*;
use rsbmp::target::{CortexMAccess, TOPT_NON_HALTING_MEM_IO};

/// One row of the AT32 part table.
///
/// A row is the set of parts that share one geometry: the C driver had one `case` per
/// package variant, all falling through to the same `stm32f1_add_flash()` calls.
struct At32Row {
    /// Primary series id ([`AT32_IDCODE_SERIES_MASK`] of the `IDCODE`).
    series0: u32,
    /// Optional second series id (`AT32F421` covers two series ids).
    series1: u32,
    /// Project ids the row covers ([`AT32_PROJECT_ID`] of the factory UID).
    project_ids: [u8; 2],
    /// Part ids the row covers; empty matches any id (the read protected parts).
    ids: &'static [u16],
    /// Name the driver publishes to GDB.
    name: &'static str,
    /// Flash of the first bank in KiB, 0 when the locked part does not expose its flash.
    flash_kib: u16,
    /// Page size in KiB (`1` or `2`).
    page_kib: u8,
    /// SRAM in KiB.
    ram_kib: u8,
    /// The part has a second bank of `flash` bytes at [`FLASH_BANK2_BASE`].
    dual_bank: bool,
    /// The core allows SRAM access without halting.
    non_halting: bool,
}

/// The AT32 part table, in the order of the C `at32f40x_probe()` dispatch.
const AT32_ROWS: &[At32Row] = &[
    // AT32F403 (200 MHz): 96 KiB SRAM, no SRAM access without halting.
    At32Row {
        series0: AT32F40_SERIES,
        series1: AT32F40_SERIES,
        project_ids: [AT32_PROJECT_F403, AT32_PROJECT_F403],
        ids: &[0x0240, 0x0241, 0x0242, 0x0243, 0x024e],
        name: DRIVER_NAME_AT32F403,
        flash_kib: 256,
        page_kib: 2,
        ram_kib: 96,
        dual_bank: false,
        non_halting: false,
    },
    At32Row {
        series0: AT32F40_SERIES,
        series1: AT32F40_SERIES,
        project_ids: [AT32_PROJECT_F403, AT32_PROJECT_F403],
        ids: &[0x02c8, 0x02c9, 0x02ca, 0x02cb, 0x02cd],
        name: DRIVER_NAME_AT32F403,
        flash_kib: 512,
        page_kib: 2,
        ram_kib: 96,
        dual_bank: false,
        non_halting: false,
    },
    At32Row {
        series0: AT32F40_SERIES,
        series1: AT32F40_SERIES,
        project_ids: [AT32_PROJECT_F403, AT32_PROJECT_F403],
        ids: &[0x0344, 0x0345, 0x0346, 0x0347, 0x034c],
        name: DRIVER_NAME_AT32F403,
        flash_kib: 512,
        page_kib: 2,
        ram_kib: 96,
        dual_bank: true,
        non_halting: false,
    },
    // AT32F403A/F407: the same three geometries, plus SRAM access without halting.
    At32Row {
        series0: AT32F40_SERIES,
        series1: AT32F40_SERIES,
        project_ids: [AT32_PROJECT_F403A, AT32_PROJECT_F407],
        ids: &[0x0240, 0x0241, 0x0242, 0x0243, 0x0249, 0x024a, 0x0254],
        name: DRIVER_NAME_AT32F403A_407,
        flash_kib: 256,
        page_kib: 2,
        ram_kib: 96,
        dual_bank: false,
        non_halting: true,
    },
    At32Row {
        series0: AT32F40_SERIES,
        series1: AT32F40_SERIES,
        project_ids: [AT32_PROJECT_F403A, AT32_PROJECT_F407],
        ids: &[0x02cd, 0x02ce, 0x02cf, 0x02d0, 0x02d1, 0x02d2],
        name: DRIVER_NAME_AT32F403A_407,
        flash_kib: 512,
        page_kib: 2,
        ram_kib: 96,
        dual_bank: false,
        non_halting: true,
    },
    At32Row {
        series0: AT32F40_SERIES,
        series1: AT32F40_SERIES,
        project_ids: [AT32_PROJECT_F403A, AT32_PROJECT_F407],
        ids: &[0x0344, 0x0345, 0x0346, 0x0347, 0x034b, 0x034c, 0x0353],
        name: DRIVER_NAME_AT32F403A_407,
        flash_kib: 512,
        page_kib: 2,
        ram_kib: 96,
        dual_bank: true,
        non_halting: true,
    },
    // A read protected F403A/F407 answers no part id the table could match.
    At32Row {
        series0: AT32F40_SERIES,
        series1: AT32F40_SERIES,
        project_ids: [AT32_PROJECT_LOCKED, AT32_PROJECT_LOCKED],
        ids: &[],
        name: DRIVER_NAME_AT32F403A_407_LOCKED,
        flash_kib: 0,
        page_kib: 0,
        ram_kib: 96,
        dual_bank: false,
        non_halting: true,
    },
    // AT32F415: 32 KiB SRAM, 2 KiB pages.
    At32Row {
        series0: AT32F41_SERIES,
        series1: AT32F41_SERIES,
        project_ids: [AT32_PROJECT_F415, AT32_PROJECT_F415],
        ids: &[0x0240, 0x0241, 0x0242, 0x0243, 0x024c],
        name: DRIVER_NAME_AT32F415,
        flash_kib: 256,
        page_kib: 2,
        ram_kib: 32,
        dual_bank: false,
        non_halting: false,
    },
    At32Row {
        series0: AT32F41_SERIES,
        series1: AT32F41_SERIES,
        project_ids: [AT32_PROJECT_F415, AT32_PROJECT_F415],
        ids: &[0x01c4, 0x01c5, 0x01c6, 0x01c7, 0x01cd],
        name: DRIVER_NAME_AT32F415,
        flash_kib: 128,
        page_kib: 2,
        ram_kib: 32,
        dual_bank: false,
        non_halting: false,
    },
    At32Row {
        series0: AT32F41_SERIES,
        series1: AT32F41_SERIES,
        project_ids: [AT32_PROJECT_F415, AT32_PROJECT_F415],
        ids: &[0x0108, 0x0109, 0x010a],
        name: DRIVER_NAME_AT32F415,
        flash_kib: 64,
        page_kib: 2,
        ram_kib: 32,
        dual_bank: false,
        non_halting: false,
    },
    // AT32F413: 32 KiB SRAM, 1 KiB pages below the 256 KiB part.
    At32Row {
        series0: AT32F41_SERIES,
        series1: AT32F41_SERIES,
        project_ids: [AT32_PROJECT_F413, AT32_PROJECT_F413],
        ids: &[0x0240, 0x0242, 0x0244, 0x0247],
        name: DRIVER_NAME_AT32F413,
        flash_kib: 256,
        page_kib: 2,
        ram_kib: 32,
        dual_bank: false,
        non_halting: false,
    },
    At32Row {
        series0: AT32F41_SERIES,
        series1: AT32F41_SERIES,
        project_ids: [AT32_PROJECT_F413, AT32_PROJECT_F413],
        ids: &[0x01c1, 0x01c3, 0x01c5, 0x01ca],
        name: DRIVER_NAME_AT32F413,
        flash_kib: 128,
        page_kib: 1,
        ram_kib: 32,
        dual_bank: false,
        non_halting: false,
    },
    At32Row {
        series0: AT32F41_SERIES,
        series1: AT32F41_SERIES,
        project_ids: [AT32_PROJECT_F413, AT32_PROJECT_F413],
        ids: &[0x0106],
        name: DRIVER_NAME_AT32F413,
        flash_kib: 64,
        page_kib: 1,
        ram_kib: 32,
        dual_bank: false,
        non_halting: false,
    },
    // AT32F421: 1 KiB pages, 16 or 8 KiB SRAM depending on the part.
    At32Row {
        series0: AT32F421_SERIES,
        series1: AT32F421_SERIES_16K,
        project_ids: [AT32_PROJECT_F421, AT32_PROJECT_F421],
        ids: &[0x0100, 0x0101, 0x0102, 0x0103, 0x0104, 0x0105, 0x0112, 0x0115],
        name: DRIVER_NAME_AT32F421,
        flash_kib: 64,
        page_kib: 1,
        ram_kib: 16,
        dual_bank: false,
        non_halting: false,
    },
    At32Row {
        series0: AT32F421_SERIES,
        series1: AT32F421_SERIES_16K,
        project_ids: [AT32_PROJECT_F421, AT32_PROJECT_F421],
        ids: &[0x0086, 0x0087, 0x0088, 0x0089, 0x008a, 0x008b, 0x0093],
        name: DRIVER_NAME_AT32F421,
        flash_kib: 32,
        page_kib: 1,
        ram_kib: 16,
        dual_bank: false,
        non_halting: false,
    },
    At32Row {
        series0: AT32F421_SERIES,
        series1: AT32F421_SERIES_16K,
        project_ids: [AT32_PROJECT_F421, AT32_PROJECT_F421],
        ids: &[0x000c, 0x000d, 0x000e, 0x000f, 0x0010, 0x0011, 0x0014, 0x0016],
        name: DRIVER_NAME_AT32F421,
        flash_kib: 16,
        page_kib: 1,
        ram_kib: 8,
        dual_bank: false,
        non_halting: false,
    },
    // AT32F425: 20 KiB SRAM, SRAM access without halting.
    At32Row {
        series0: AT32F425_SERIES,
        series1: AT32F425_SERIES,
        project_ids: [AT32_PROJECT_F425, AT32_PROJECT_F425],
        ids: &[0x0100, 0x0103, 0x0106, 0x0109, 0x010c, 0x010f, 0x0112],
        name: DRIVER_NAME_AT32F425,
        flash_kib: 64,
        page_kib: 1,
        ram_kib: 20,
        dual_bank: false,
        non_halting: true,
    },
    At32Row {
        series0: AT32F425_SERIES,
        series1: AT32F425_SERIES,
        project_ids: [AT32_PROJECT_F425, AT32_PROJECT_F425],
        ids: &[0x0081, 0x0084, 0x0087, 0x008a, 0x008d, 0x0090, 0x0093],
        name: DRIVER_NAME_AT32F425,
        flash_kib: 32,
        page_kib: 1,
        ram_kib: 20,
        dual_bank: false,
        non_halting: true,
    },
];

/// One row of the MM32 part table.
struct Mm32Row {
    /// Whole `DBGMCU_IDCODE` word of the part.
    id: u32,
    /// Name the driver publishes to GDB.
    name: &'static str,
    flash_kib: u16,
    /// Main SRAM at [`SRAM_BASE`], in KiB.
    ram_kib: u8,
    /// Second SRAM at [`MM32F52_RAM2_BASE`] in KiB, 0 for the parts that have none.
    ram2_kib: u8,
}

/// The MM32L0 parts (Cortex-M0), which report their id in their own `DBGMCU` block.
const MM32L0_ROWS: &[Mm32Row] = &[
    Mm32Row {
        id: MM32L0_ID_MM32L07X,
        name: DRIVER_NAME_MM32L07X,
        flash_kib: 128,
        ram_kib: 8,
        ram2_kib: 0,
    },
    Mm32Row {
        id: MM32L0_ID_MM32SPIN05,
        name: DRIVER_NAME_MM32SPIN05,
        flash_kib: 32,
        ram_kib: 4,
        ram2_kib: 0,
    },
    Mm32Row {
        id: MM32L0_ID_MM32SPIN27,
        name: DRIVER_NAME_MM32SPIN27,
        flash_kib: 128,
        ram_kib: 12,
        ram2_kib: 0,
    },
];

/// The MM32F3 parts (Star-MC1), which report their id in their own `DBGMCU` block.
const MM32F3_ROWS: &[Mm32Row] = &[
    Mm32Row {
        id: MM32F3_ID_MM32F327,
        name: DRIVER_NAME_MM32F327,
        flash_kib: 512,
        ram_kib: 128,
        ram2_kib: 0,
    },
    Mm32Row {
        id: MM32F3_ID_MM32F52,
        name: DRIVER_NAME_MM32F52,
        flash_kib: 256,
        ram_kib: 32,
        ram2_kib: 128,
    },
];

/// Probe the Artery AT32F40x line (C `at32f40x_probe()`).
///
/// Artery identifies its parts by the complete `IDCODE` word (series and part number) plus
/// the highest byte of the factory UID, so a part only matches a row on all three.
pub fn probe_at32f40x(cm: &mut CortexMAccess) -> Option<F1Driver> {
    let cpuid_partno = cm.cpuid() & CORTEX_CPUID_PARTNO_MASK;
    // The Artery parts use Cortex-M4 cores.
    if cpuid_partno != CORTEX_M4 {
        return None;
    }

    let idcode = cm.read32(F1_DBGMCU_IDCODE);
    let series = idcode & AT32_IDCODE_SERIES_MASK;
    let part_id = (idcode & AT32_IDCODE_PART_MASK) as u16;
    // The project id is a single byte, and the read protection erases it to 0xff.
    let mut project = [0u8; 1];
    cm.mem_read(&mut project, AT32_PROJECT_ID);
    let project_id = project[0];

    let row = AT32_ROWS.iter().find(|row| {
        (row.series0 == series || row.series1 == series)
            && (row.project_ids[0] == project_id || row.project_ids[1] == project_id)
            && (row.ids.is_empty() || row.ids.contains(&part_id))
    })?;

    // Artery programs 32 bits at a time; only the parts whose core allows it advertise
    // memory I/O without halting the target.
    let options = TOPT_32BIT_WRITES
        | if row.non_halting {
            TOPT_NON_HALTING_MEM_IO
        } else {
            0
        };
    let mut driver = F1Driver::new(
        row.name,
        part_id,
        cpuid_partno,
        F1_DBGMCU_CONFIG,
        true,
        row.dual_bank,
        options,
    )
    .with_ram(usize::from(row.ram_kib) * KIB);

    // A read protected part answers no flash geometry, so it gets no flash region at all
    // and can only be recovered with `mon erase_mass`.
    if row.flash_kib != 0 {
        let flash = usize::from(row.flash_kib) * KIB;
        let page = usize::from(row.page_kib) * KIB;
        driver = driver.with_flash(flash, page);
        if row.dual_bank {
            driver = driver.with_bank2(flash, page);
        }
    }

    driver.finish(cm)
}

/// Probe the MM32L0 line (Cortex-M0, C `mm32l0xx_probe()`).
pub fn probe_mm32l0(cm: &mut CortexMAccess) -> Option<F1Driver> {
    let id = cm.read32(MM32L0_DBGMCU_IDCODE);
    if cm.check_error() {
        return None;
    }
    let row = MM32L0_ROWS.iter().find(|row| row.id == id)?;
    // These parts only latch the lowest lane of a write, so the access port gets a write
    // primitive that broadcasts every beat to all four lanes (C did this in the probe too).
    cm.install_mm32l0_broadcast_write();
    mm32_driver(cm, row, MM32L0_DBGMCU_CONFIG)
}

/// Probe the MM32F3 line (Star-MC1, C `mm32f3xx_probe()`).
pub fn probe_mm32f3(cm: &mut CortexMAccess) -> Option<F1Driver> {
    let id = cm.read32(MM32F3_DBGMCU_IDCODE);
    if cm.check_error() {
        return None;
    }
    let row = MM32F3_ROWS.iter().find(|row| row.id == id)?;
    mm32_driver(cm, row, MM32F3_DBGMCU_CONFIG)
}

/// Build the driver of an MM32 part: one bank, 1 KiB pages and 16-bit program steps, in
/// the `DBGMCU` block the line keeps its freeze bits in.
fn mm32_driver(cm: &mut CortexMAccess, row: &Mm32Row, dbgmcu_config: u32) -> Option<F1Driver> {
    let mut driver = F1Driver::new(
        row.name,
        (row.id & IDCODE_DEV_ID_MASK) as u16,
        cm.cpuid() & CORTEX_CPUID_PARTNO_MASK,
        dbgmcu_config,
        false,
        false,
        0,
    )
    .with_ram(usize::from(row.ram_kib) * KIB)
    .with_flash(usize::from(row.flash_kib) * KIB, MM32_PAGE);

    if row.ram2_kib != 0 {
        driver = driver.with_extra_ram(MM32F52_RAM2_BASE, usize::from(row.ram2_kib) * KIB);
    }

    driver.finish(cm)
}
