/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (c) 2021-2023 Fabrice Prost-Boucle <fabalthazar@falbalab.fr>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the part table of blackmagic/src/target/stm32g0.c (the switch of
 * `stm32g0_probe()`), made for the Swindle firmware. This file is derived from that C
 * source and stays under the same license as the original Black Magic code reproduced
 * in `lib.rs`.
 */

//! Part table of the STM32G0, STM32C0 and STM32U0 families: the DBGMCU `IDCODE` device
//! ids, the SRAM size each part has, how its flash size is found and the driver name GDB
//! sees.
//!
//! Single bank devices are mapped with their *maximal* flash capacity, so a part with
//! less flash than the family maximum can still be programmed when it is really a bigger
//! die (that is what the C driver does). The dual bank G0B/C and the U0 parts instead
//! report their real size in a flash-size register, which the probe reads.

use crate::registers::*;
use rs_stm32_common::dbgmcu::idcode_dev_id;
use rsbmp::target::CortexMAccess;

// Device ids, as reported by the DBGMCU `IDCODE`.
//
// The underscores in these names stand for slashes: `PART_STM32G03_4` refers to the
// G03/G04, and so on.

/// STM32C011.
pub const PART_STM32C011: u16 = 0x443;
/// STM32C031.
pub const PART_STM32C031: u16 = 0x453;
/// STM32C051.
pub const PART_STM32C051: u16 = 0x44c;
/// STM32C071.
pub const PART_STM32C071: u16 = 0x493;
/// STM32C091/C092.
pub const PART_STM32C09X: u16 = 0x44d;
/// STM32G03/G04.
pub const PART_STM32G03_4: u16 = 0x466;
/// STM32G05/G06.
pub const PART_STM32G05_6: u16 = 0x456;
/// STM32G07/G08.
pub const PART_STM32G07_8: u16 = 0x460;
/// STM32G0B1/G0C1.
pub const PART_STM32G0B_C: u16 = 0x467;
/// STM32U031.
pub const PART_STM32U031: u16 = 0x4590;
/// STM32U073/083.
pub const PART_STM32U0X3: u16 = 0x4890;

/// One entry of the part table.
#[derive(Debug, Clone, Copy)]
pub struct Part {
    /// Device id as the DBGMCU `IDCODE` reports it.
    pub id: u16,
    /// Driver name, NUL terminated for the C `target_s`.
    pub name: &'static str,
    /// Size of the main SRAM region at [`SRAM_BASE`], in KiB.
    pub ram_kib: u16,
    /// Flash size in KiB, for the parts that are mapped with their family maximum.
    pub flash_max_kib: u16,
    /// Register carrying the real flash size in KiB, for the parts that report it (0 when fixed).
    pub flash_size_reg: u32,
    /// Only `mon uid` is offered (`mon option`, `mon erase_bank` and `mon irreversible`
    /// are missing on the STM32U0 parts).
    pub reduced_commands: bool,
}

/// A single-bank part mapped with its family's maximal flash size.
const fn part(id: u16, name: &'static str, ram_kib: u16, flash_max_kib: u16) -> Part {
    Part {
        id,
        name,
        ram_kib,
        flash_max_kib,
        flash_size_reg: 0,
        reduced_commands: false,
    }
}

/// A part that reports its real flash size in `flash_size_reg`, in KiB.
const fn reporting_part(
    id: u16,
    name: &'static str,
    ram_kib: u16,
    flash_size_reg: u32,
    reduced_commands: bool,
) -> Part {
    Part {
        id,
        name,
        ram_kib,
        flash_max_kib: 0,
        flash_size_reg,
        reduced_commands,
    }
}

/// The parts this driver drives (C `stm32g0_probe`).
pub static PARTS: [Part; 11] = [
    part(PART_STM32G03_4, "STM32G03/4\0", 8, 64),
    part(PART_STM32C011, "STM32C011\0", 6, 32),
    part(PART_STM32C031, "STM32C031\0", 12, 32),
    part(PART_STM32C051, "STM32C051\0", 12, 64),
    part(PART_STM32C071, "STM32C071\0", 24, 128),
    // TODO: the C driver cannot tell the C091 and the C092 apart either.
    part(PART_STM32C09X, "STM32C09x\0", 36, 256),
    part(PART_STM32G05_6, "STM32G05/6\0", 18, 64),
    part(PART_STM32G07_8, "STM32G07/8\0", 36, 128),
    reporting_part(PART_STM32G0B_C, "STM32G0B/C\0", 144, FLASH_SIZE_REG, false),
    reporting_part(PART_STM32U031, "STM32U031\0", 12, U031_FLASH_SIZE_REG, true),
    reporting_part(PART_STM32U0X3, "STM32U0x3\0", 40, U0X3_FLASH_SIZE_REG, true),
];

/// Entry of [`PARTS`] with device id `id`.
pub fn part_by_id(id: u16) -> Option<&'static Part> {
    PARTS.iter().find(|part| part.id == id)
}

/// Resolve the probed part, or `None` when it is not one of ours.
///
/// The G03/G04 shares its first-level id with the C011/C031, so the driver re-reads the
/// `DEV_ID` field of the DBGMCU `IDCODE` for that one — exactly like the C probe.
pub fn resolve(cm: &CortexMAccess) -> Option<&'static Part> {
    let id = cm.part_id() as u16;
    let id = if id == PART_STM32G03_4 {
        idcode_dev_id(cm, DBGMCU_IDCODE)
    } else {
        id
    };
    part_by_id(id)
}
