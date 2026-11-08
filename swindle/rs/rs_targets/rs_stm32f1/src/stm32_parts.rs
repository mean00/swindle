/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the part table of blackmagic/src/target/stm32f1.c (the `stm32f1_probe()`
 * and `gd32f1_probe()` switches), made for the Swindle firmware. This file is derived
 * from that C source and stays under the same license as the original Black Magic
 * code reproduced below.
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

//! Part table of the STM32F0/F1/F3 families and their GD32 clones: the DBGMCU `IDCODE`
//! device ids, the memory geometry each density group reports and the driver names GDB
//! sees.
//!
//! Parts within a density group share their SRAM size, flash size and page size, so the
//! probe only has to name the group it matched. The values are the ones documented in
//! `stm32f1_probe()` and `gd32f1_probe()`, where the same part groups stand as bare
//! literals. The F2/F4/F7 ids stay next to the registers of that family, in
//! [`crate::registers`].

/// Width of the `DEV_ID` field of the DBGMCU `IDCODE` register.
pub use rs_stm32_common::registers::{AP_IDR_REVISION_SHIFT, IDCODE_DEV_ID_MASK, KIB};
/// Clone F1 parts report core revision 2 and up where genuine ones report 1.
pub const F1_MIN_CLONE_CORE_REVISION: u32 = 2;

// Device ids, as reported by the DBGMCU `IDCODE`.

/// CS clones of the F1 medium density parts.
pub const PART_CS_CLONE: u16 = 0x29b;
/// STM32F1 medium density.
pub const PART_STM32F1_MEDIUM: u16 = 0x410;
/// STM32F1 low density.
pub const PART_STM32F1_LOW: u16 = 0x412;
/// STM32F1 value line, low and medium density.
pub const PART_STM32F1_VALUE_LM: u16 = 0x420;
/// STM32F1 high density.
pub const PART_STM32F1_HIGH: u16 = 0x414;
/// STM32F1 connectivity line.
pub const PART_STM32F1_CONNECTIVITY: u16 = 0x418;
/// STM32F1 value line, high density.
pub const PART_STM32F1_VALUE_HD: u16 = 0x428;
/// STM32F1 XL density (two flash banks).
pub const PART_STM32F1_XL: u16 = 0x430;
/// STM32F303x6/8 and STM32F328.
pub const PART_STM32F303X6X8: u16 = 0x438;
/// STM32F30x.
pub const PART_STM32F30X: u16 = 0x422;
/// STM32F303xD/E and STM32F398xE.
pub const PART_STM32F303XDE: u16 = 0x446;
/// STM32F37x.
pub const PART_STM32F37X: u16 = 0x432;
/// STM32F302C8.
pub const PART_STM32F302C8: u16 = 0x439;
/// STM32F03, STM32F030x[4|6].
pub const PART_STM32F03: u16 = 0x444;
/// STM32F04, STM32F070x6.
pub const PART_STM32F04: u16 = 0x445;
/// STM32F05, STM32F030x8.
pub const PART_STM32F05: u16 = 0x440;
/// STM32F07, STM32F070xb.
pub const PART_STM32F07: u16 = 0x448;
/// STM32F09, STM32F030xc.
pub const PART_STM32F09: u16 = 0x442;

// Device ids of the GD32 parts (C `gd32f1_probe`). They reuse STM32 id values for
// different parts, so they are named after the GD32 the id belongs to.

/// GD32F30x high density.
pub const PART_GD32F30X_HD: u16 = 0x414;
/// GD32F30x extra density.
pub const PART_GD32F30X_XD: u16 = 0x430;
/// GD32F2 connectivity line.
pub const PART_GD32F2: u16 = 0x418;
/// GD32 medium density: the core decides between GD32F1, GD32F3 and GD32E230.
pub const PART_GD32_MEDIUM: u16 = 0x410;
/// GD32E50x connectivity line.
pub const PART_GD32E5: u16 = 0x444;

/// The GD32 `FLASHSIZE` signature holds the flash size in KiB in its low halfword and
/// the SRAM size in KiB above it.
pub const GD32_FLASH_KIB_MASK: u32 = 0xffff;
pub const GD32_RAM_KIB_SHIFT: u32 = 16;
/// Flash size from which the GD32 parts have two banks.
pub const GD32_DUAL_BANK_MIN_KIB: usize = 512;

// Memory geometry of the density groups, in bytes.

/// SRAM, flash and page size of the F1 low and medium density parts (and their clones).
pub const F1_LM_DENSITY_RAM: usize = 0x5000;
pub const F1_LM_DENSITY_FLASH: usize = 0x2_0000;
pub const F1_LM_DENSITY_PAGE: usize = 0x400;
/// SRAM of the F1 XL density parts; their flash is two [`F1_HD_DENSITY_FLASH`] banks.
pub const F1_XL_DENSITY_RAM: usize = 0x1_8000;
/// SRAM, flash and page size of the F1 high density, connectivity line and value line
/// high density parts.
pub const F1_HD_DENSITY_RAM: usize = 0x1_0000;
pub const F1_HD_DENSITY_FLASH: usize = 0x8_0000;
pub const F1_HD_DENSITY_PAGE: usize = 0x800;
/// SRAM, flash and page size of the STM32F3 parts.
pub const F3_RAM: usize = 0x1_0000;
pub const F3_FLASH: usize = 0x8_0000;
pub const F3_PAGE: usize = 0x800;
/// SRAM, flash and page size of the STM32F0 parts, per density group.
pub const F03_RAM: usize = 0x1000;
pub const F03_FLASH: usize = 0x8000;
pub const F03_PAGE: usize = 0x400;
pub const F04_RAM: usize = 0x1800;
pub const F04_FLASH: usize = 0x8000;
pub const F04_PAGE: usize = 0x400;
pub const F05_RAM: usize = 0x2000;
pub const F05_FLASH: usize = 0x1_0000;
pub const F05_PAGE: usize = 0x400;
pub const F07_RAM: usize = 0x4000;
pub const F07_FLASH: usize = 0x2_0000;
pub const F07_PAGE: usize = 0x800;
pub const F09_RAM: usize = 0x8000;
pub const F09_FLASH: usize = 0x4_0000;
pub const F09_PAGE: usize = 0x800;

/// Page size of the GD32 medium density parts.
pub const GD32_MEDIUM_PAGE: usize = 0x400;
/// Page size of the GD32F2/F3 high and extra density parts.
pub const GD32_HD_PAGE: usize = 0x800;
/// Page size of the second bank of a GD32F3 extra density part.
pub const GD32_BANK2_PAGE: usize = 0x1000;
/// Page size of the GD32E5 parts.
pub const GD32E5_PAGE: usize = 0x2000;

// Device ids and geometry of the MM32 parts (C `mm32l0xx_probe()` / `mm32f3xx_probe()`).
// Their `DBGMCU_IDCODE` is a full 32-bit device id, not the `DEV_ID` field of an ARM part.

/// MindMotion MM32L07x (Cortex-M0).
pub const MM32L0_ID_MM32L07X: u32 = 0xcc56_8091;
/// MindMotion MM32SPIN05 (Cortex-M0).
pub const MM32L0_ID_MM32SPIN05: u32 = 0xcc44_60b1;
/// MindMotion MM32SPIN27 (Cortex-M0).
pub const MM32L0_ID_MM32SPIN27: u32 = 0xcc56_a097;
/// MindMotion MM32F327 (Star-MC1).
pub const MM32F3_ID_MM32F327: u32 = 0xcc9a_a0e7;
/// MindMotion MM32F52 (Star-MC1), with a second SRAM region.
pub const MM32F3_ID_MM32F52: u32 = 0x4d4d_0800;

/// Erase block size of every MM32 part.
pub const MM32_PAGE: usize = 0x400;

/// Page size of the AT32 parts with 2 KiB pages (F403/F403A/F407, F413 medium, F415).
pub const AT32_PAGE_2K: usize = 2 * KIB;
/// Page size of the AT32 parts with 1 KiB pages (F413 small, F421, F425).
pub const AT32_PAGE_1K: usize = KIB;
/// SRAM of every AT32F403/F403A/F407 part.
pub const AT32F40_RAM: usize = 96 * KIB;
/// SRAM of the AT32F413/F415 parts.
pub const AT32F41_RAM: usize = 32 * KIB;
/// SRAM of every AT32F425 part.
pub const AT32F425_RAM: usize = 20 * KIB;

// Driver names, as the C driver publishes them to GDB (NUL-terminated).

/// Names of the AT32 lines (C `at32f403_detect()` and friends).
pub const DRIVER_NAME_AT32F403: &str = "AT32F403\0";
pub const DRIVER_NAME_AT32F403A_407: &str = "AT32F403A/407\0";
/// A locked F403A/F407 only answers its project id, so its geometry is unknown.
pub const DRIVER_NAME_AT32F403A_407_LOCKED: &str = "AT32F403A/407 (locked)\0";
pub const DRIVER_NAME_AT32F413: &str = "AT32F413\0";
pub const DRIVER_NAME_AT32F415: &str = "AT32F415\0";
pub const DRIVER_NAME_AT32F421: &str = "AT32F421\0";
pub const DRIVER_NAME_AT32F425: &str = "AT32F425\0";
/// Names of the MM32 parts (C `mm32l0xx_probe()` / `mm32f3xx_probe()`).
pub const DRIVER_NAME_MM32L07X: &str = "MM32L07x\0";
pub const DRIVER_NAME_MM32SPIN05: &str = "MM32SPIN05\0";
pub const DRIVER_NAME_MM32SPIN27: &str = "MM32SPIN27\0";
pub const DRIVER_NAME_MM32F327: &str = "MM32F327\0";
pub const DRIVER_NAME_MM32F52: &str = "MM32F52\0";
/// Name of the CH32F103 (C `ch32f1_probe()`).
pub const DRIVER_NAME_CH32F1: &str = "CH32F1 medium density (stm32f1 clone)\0";

// Driver names, as the C driver publishes them to GDB (NUL-terminated).

/// Name of the F1 clones, which the probe only picks once it has seen the core revision.
pub const DRIVER_NAME_F1_CLONE: &str = "Clone STM32F1 medium density\0";
/// Name of the F1 low and medium density parts.
pub const DRIVER_NAME_F1_LM: &str = "STM32F1 L/M density\0";
/// Name of the F1 high density, connectivity line and value line high density parts.
pub const DRIVER_NAME_F1_VL: &str = "STM32F1 VL density\0";
/// Name of the F1 XL density parts.
pub const DRIVER_NAME_F1_XL: &str = "STM32F1 XL density\0";
/// Name of the STM32F3 parts.
pub const DRIVER_NAME_F3: &str = "STM32F3\0";
/// Names of the STM32F0 parts, per density group.
pub const DRIVER_NAME_F03: &str = "STM32F03\0";
pub const DRIVER_NAME_F04: &str = "STM32F04/F070x6\0";
pub const DRIVER_NAME_F05: &str = "STM32F05/F030x8\0";
pub const DRIVER_NAME_F07: &str = "STM32F07\0";
pub const DRIVER_NAME_F09: &str = "STM32F09/F030xc\0";
/// Name of the GD32F30x high and extra density parts.
pub const DRIVER_NAME_GD32F3_HDXD: &str = "GD32F3 HD/XD\0";
/// Name of the GD32 medium density parts with a Cortex-M4 core.
pub const DRIVER_NAME_GD32F3_MD: &str = "GD32F3 MD\0";
/// Name of the GD32F2 connectivity line parts.
pub const DRIVER_NAME_GD32F2: &str = "GD32F2\0";
/// Name of the GD32 medium density parts with a Cortex-M23 core.
pub const DRIVER_NAME_GD32E230: &str = "GD32E230\0";
/// Name of the GD32 medium density parts with a Cortex-M3 core.
pub const DRIVER_NAME_GD32F1: &str = "GD32F1\0";
/// Name of the GD32E50x connectivity line parts.
pub const DRIVER_NAME_GD32E5: &str = "GD32E5\0";
