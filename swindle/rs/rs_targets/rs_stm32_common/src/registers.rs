/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2017, 2018 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (c) 2021-2023 Fabrice Prost-Boucle <fabalthazar@falbalab.fr>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the constants shared by the memory maps and `mon` output of
 * blackmagic/src/target/stm32f1.c, stm32f4.c and stm32g0.c (bank bases, SFN/UID sizes,
 * the core part numbers the probes branch on and the FPEC unlock keys), made for the
 * Swindle firmware. This file is derived from those C sources and stays under the same
 * licenses as the original Black Magic code reproduced in `lib.rs`.
 */

//! Constants every STM32 family register map in this workspace repeats.
//!
//! Keeping them here means a family module only has to spell out what is specific to
//! its flash controller and its own part ids.

/// Base address of the first flash bank (every STM32 family here).
pub const FLASH_BANK1_BASE: u32 = 0x0800_0000;
/// Base address of the SRAM.
pub const SRAM_BASE: u32 = 0x2000_0000;

/// Bytes in one KiB, the unit the part tables are written in.
pub const KIB: usize = 1024;

/// Width of the `DEV_ID` field of the DBGMCU `IDCODE` register.
pub const IDCODE_DEV_ID_MASK: u32 = 0xfff;
/// Field of the access port `IDR` that carries the core revision.
pub const AP_IDR_REVISION_SHIFT: u32 = 28;

/// Length in bytes of the factory UID (96 bits on every STM32 part).
pub const UID_LEN: usize = 12;

/// Hex digits of a 32-bit address.
pub const ADDR_HEX_DIGITS: usize = 8;
/// Hex digits of a 16-bit option halfword.
pub const HALFWORD_HEX_DIGITS: usize = 4;
/// Hex digits of any 32-bit word.
pub const WORD_HEX_DIGITS: usize = 8;
/// Hex digits of one 32-bit UID word.
pub const UID_WORD_HEX_DIGITS: usize = WORD_HEX_DIGITS;

/// `CPUID` part number field / mask.
pub const CORTEX_CPUID_PARTNO_MASK: u32 = 0xfff0_0000;
pub const CORTEX_M0: u32 = 0xc200_0000;
pub const CORTEX_M3: u32 = 0xc230_0000;
pub const CORTEX_M4: u32 = 0xc240_0000;
pub const CORTEX_M23: u32 = 0xc270_0000;
pub const CORTEX_M33: u32 = 0xd210_0000;

/// First FPEC unlock key (the same pair on every STM32 flash controller).
pub const FPEC_KEY1: u32 = 0x4567_0123;
/// Second FPEC unlock key.
pub const FPEC_KEY2: u32 = 0xcdef_89ab;
