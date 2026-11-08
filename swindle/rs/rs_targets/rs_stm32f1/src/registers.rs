/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2017, 2018 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the register maps of blackmagic/src/target/stm32f1.c
 * (FPEC and debug MCU blocks), made for the Swindle firmware.
 */

//! Register map of the STM32F1-style flash controller (FPEC), DBGMCU blocks,
//! and related constants for STM32F1/F0/F3 and GD32/AT32/MM32 clones.

#![allow(unused_imports)]

pub use rs_stm32_common::registers::*;

/// Base address of the second flash bank.
pub const FLASH_BANK2_BASE: u32 = 0x0808_0000;
/// Address at which the flash switches from bank 1 to bank 2.
pub const FLASH_BANK_SPLIT: u32 = FLASH_BANK2_BASE;

/// GD32 flash/RAM size signature (`[31:16]` = RAM KiB, `[15:0]` = flash KiB).
pub const GD32FX_FLASHSIZE: u32 = 0x1fff_f7e0;

// ---------------------------------------------------------------------------
// FPEC (Flash Program/Erase Controller)
// ---------------------------------------------------------------------------

/// `FLASH_ACR` base; the register block is shared by every F1-style family in this crate.
pub const FPEC_BASE: u32 = 0x4002_2000;

#[repr(C)]
pub struct FlashRegisters {
    pub acr: u32,       // 0x00 Access control
    pub keyr: u32,      // 0x04 FPEC key
    pub optkeyr: u32,   // 0x08 Option byte key
    pub sr: u32,        // 0x0C Status
    pub cr: u32,        // 0x10 Control
    pub ar: u32,        // 0x14 Address
    pub reserved: u32,  // 0x18 Reserved
    pub obr: u32,       // 0x1C Option byte
    pub wrpr: u32,      // 0x20 Write protection
}

// Datasheet tripwire
const _: () = assert!(::core::mem::offset_of!(FlashRegisters, wrpr) == 0x20);

/// Address of a [`FlashRegisters`] member in the target: `FPEC_BASE + offsetof(member)`.
/// With bank offset: `FPEC_BASE + bank_offset + offsetof(member)`.
macro_rules! flash_reg {
    ($member:ident) => {
        $crate::registers::FPEC_BASE + ::core::mem::offset_of!($crate::registers::FlashRegisters, $member) as u32
    };
    ($bank_offset:expr, $member:ident) => {
        $crate::registers::FPEC_BASE + $bank_offset + ::core::mem::offset_of!($crate::registers::FlashRegisters, $member) as u32
    };
}
pub(crate) use flash_reg;

pub const FPEC_ACCESS_CTRL: u32 = flash_reg!(acr);
pub const FPEC_KEY: u32 = flash_reg!(keyr);
pub const FPEC_OPTION_KEY: u32 = flash_reg!(optkeyr);
pub const FPEC_STATUS: u32 = flash_reg!(sr);
pub const FPEC_CTRL: u32 = flash_reg!(cr);
pub const FPEC_ADDR: u32 = flash_reg!(ar);
pub const FPEC_OPTION_BYTE: u32 = flash_reg!(obr);
pub const FPEC_WRITE_PROT: u32 = flash_reg!(wrpr);

/// `CR`: launch the option byte loading.
pub const FPEC_OBL_LAUNCH: u32 = 1 << 13;
/// `CR`: option byte write enable.
pub const FPEC_OPTWRE: u32 = 1 << 9;
/// `CR`: the controller is locked.
pub const FPEC_LOCK: u32 = 1 << 7;
/// `CR`: start an erase / program operation.
pub const FPEC_STRT: u32 = 1 << 6;
/// `CR`: option byte erase.
pub const FPEC_OPTER: u32 = 1 << 5;
/// `CR`: option byte program.
pub const FPEC_OPTPG: u32 = 1 << 4;
/// `CR`: mass erase.
pub const FPEC_MER: u32 = 1 << 2;
/// `CR`: page erase.
pub const FPEC_PER: u32 = 1 << 1;
/// `CR`: flash program.
pub const FPEC_PG: u32 = 1 << 0;

/// `OBR`: the read protection is active.
pub const FPEC_OPTION_BYTE_RDPRT: u32 = 1 << 1;

/// Register offset of the first bank inside the FPEC block.
pub const FPEC_BANK1_OFFSET: u32 = 0x00;
/// Register offset of the second bank inside the FPEC block.
pub const FPEC_BANK2_OFFSET: u32 = 0x40;

/// `SR`: a program/erase operation is running.
pub const FPEC_STATUS_BSY: u32 = 1 << 0;
/// `SR`: any error flag.
pub const FPEC_STATUS_ERROR_MASK: u32 = 0x14;
/// `SR`: program error (PGERR).
pub const FPEC_STATUS_PROG_ERROR: u32 = 0x04;
/// `SR`: end of operation.
pub const FPEC_STATUS_EOP: u32 = 0x20;

// ---------------------------------------------------------------------------
// AT32 identification
// ---------------------------------------------------------------------------

pub const AT32_IDCODE_SERIES_MASK: u32 = 0xffff_f000;
pub const AT32_IDCODE_PART_MASK: u32 = 0x0000_0fff;
pub const AT32_PROJECT_ID: u32 = 0x1fff_f7f3;

pub const AT32F40_SERIES: u32 = 0x7005_0000;
pub const AT32F41_SERIES: u32 = 0x7003_0000;
pub const AT32F421_SERIES: u32 = 0x5002_0000;
pub const AT32F421_SERIES_16K: u32 = 0x5001_0000;
pub const AT32F425_SERIES: u32 = 0x5009_2000;

pub const AT32_PROJECT_F403: u8 = 0x02;
pub const AT32_PROJECT_F413: u8 = 0x04;
pub const AT32_PROJECT_F415: u8 = 0x05;
pub const AT32_PROJECT_F403A: u8 = 0x07;
pub const AT32_PROJECT_F407: u8 = 0x08;
pub const AT32_PROJECT_F421: u8 = 0x09;
pub const AT32_PROJECT_F425: u8 = 0x0f;
pub const AT32_PROJECT_LOCKED: u8 = 0xff;

/// Base address of the option bytes.
pub const FPEC_OBP_RDP: u32 = 0x1fff_f800;
/// Option byte value that keeps the flash readable on STM32F1/F3 parts.
pub const FPEC_OBP_RDP_KEY: u16 = 0x5aa5;
/// Option byte value that keeps the flash readable on the STM32F3/F0 parts (inverted key).
pub const FPEC_OBP_RDP_KEY_F3: u16 = 0x55aa;

// ---------------------------------------------------------------------------
// DBGMCU
// ---------------------------------------------------------------------------

/// `DBGMCU` of the STM32F1/F3 (Cortex-M3/M4) parts.
pub const F1_DBGMCU_BASE: u32 = 0xe004_2000;

#[repr(C)]
pub struct F1DbgmcuRegisters {
    pub idcode: u32,
    pub cr: u32,
}

const _: () = assert!(::core::mem::offset_of!(F1DbgmcuRegisters, cr) == 0x04);

macro_rules! f1_dbgmcu_reg {
    ($member:ident) => {
        $crate::registers::F1_DBGMCU_BASE + ::core::mem::offset_of!($crate::registers::F1DbgmcuRegisters, $member) as u32
    };
}
pub(crate) use f1_dbgmcu_reg;

pub const F1_DBGMCU_IDCODE: u32 = f1_dbgmcu_reg!(idcode);
pub const F1_DBGMCU_CONFIG: u32 = f1_dbgmcu_reg!(cr);

pub const F1_DBGMCU_CONFIG_DBG_SLEEP: u32 = 1 << 0;
pub const F1_DBGMCU_CONFIG_DBG_STOP: u32 = 1 << 1;
pub const F1_DBGMCU_CONFIG_DBG_STANDBY: u32 = 1 << 2;
pub const F1_DBGMCU_CONFIG_IWDG_STOP: u32 = 1 << 8;
pub const F1_DBGMCU_CONFIG_WWDG_STOP: u32 = 1 << 9;

/// `DBGMCU` of the STM32F0 parts (Cortex-M0).
pub const F0_DBGMCU_BASE: u32 = 0x4001_5800;

#[repr(C)]
pub struct F0DbgmcuRegisters {
    pub idcode: u32,
    pub cr: u32,
    pub apb1freeze: u32,
}

const _: () = assert!(::core::mem::offset_of!(F0DbgmcuRegisters, apb1freeze) == 0x08);

macro_rules! f0_dbgmcu_reg {
    ($member:ident) => {
        $crate::registers::F0_DBGMCU_BASE + ::core::mem::offset_of!($crate::registers::F0DbgmcuRegisters, $member) as u32
    };
}
pub(crate) use f0_dbgmcu_reg;

pub const F0_DBGMCU_IDCODE: u32 = f0_dbgmcu_reg!(idcode);
pub const F0_DBGMCU_CONFIG: u32 = f0_dbgmcu_reg!(cr);
pub const F0_DBGMCU_APB1FREEZE: u32 = f0_dbgmcu_reg!(apb1freeze);

pub const F0_DBGMCU_CONFIG_DBG_STOP: u32 = 1 << 1;
pub const F0_DBGMCU_CONFIG_DBG_STANDBY: u32 = 1 << 2;
pub const F0_DBGMCU_APB1FREEZE_WWDG: u32 = 1 << 11;
pub const F0_DBGMCU_APB1FREEZE_IWDG: u32 = 1 << 12;

/// `DBGMCU` of the GD32E50x/E51x (Cortex-M33) parts.
pub const GD32E5_DBGMCU_BASE: u32 = 0xe004_4000;

#[repr(C)]
pub struct Gd32e5DbgmcuRegisters {
    pub idcode: u32,
    pub cr: u32,
}

macro_rules! gd32e5_dbgmcu_reg {
    ($member:ident) => {
        $crate::registers::GD32E5_DBGMCU_BASE + ::core::mem::offset_of!($crate::registers::Gd32e5DbgmcuRegisters, $member) as u32
    };
}
pub(crate) use gd32e5_dbgmcu_reg;

pub const GD32E5_DBGMCU_IDCODE: u32 = gd32e5_dbgmcu_reg!(idcode);
pub const GD32E5_DBGMCU_CONFIG: u32 = gd32e5_dbgmcu_reg!(cr);

/// `DBGMCU` of the MindMotion MM32L0 parts (Cortex-M0).
pub const MM32L0_DBGMCU_BASE: u32 = 0x4001_3400;

#[repr(C)]
pub struct Mm32l0DbgmcuRegisters {
    pub idcode: u32,
    pub cr: u32,
}

macro_rules! mm32l0_dbgmcu_reg {
    ($member:ident) => {
        $crate::registers::MM32L0_DBGMCU_BASE + ::core::mem::offset_of!($crate::registers::Mm32l0DbgmcuRegisters, $member) as u32
    };
}
pub(crate) use mm32l0_dbgmcu_reg;

pub const MM32L0_DBGMCU_IDCODE: u32 = mm32l0_dbgmcu_reg!(idcode);
pub const MM32L0_DBGMCU_CONFIG: u32 = mm32l0_dbgmcu_reg!(cr);

/// `DBGMCU` of the MindMotion MM32F3 parts (Cortex-M3 / Star-MC1).
pub const MM32F3_DBGMCU_BASE: u32 = 0x4000_7080;

#[repr(C)]
pub struct Mm32f3DbgmcuRegisters {
    pub idcode: u32,
    pub cr: u32,
}

macro_rules! mm32f3_dbgmcu_reg {
    ($member:ident) => {
        $crate::registers::MM32F3_DBGMCU_BASE + ::core::mem::offset_of!($crate::registers::Mm32f3DbgmcuRegisters, $member) as u32
    };
}
pub(crate) use mm32f3_dbgmcu_reg;

pub const MM32F3_DBGMCU_IDCODE: u32 = mm32f3_dbgmcu_reg!(idcode);
pub const MM32F3_DBGMCU_CONFIG: u32 = mm32f3_dbgmcu_reg!(cr);

/// Second SRAM region of the MM32F52 parts, behind the main one at [`SRAM_BASE`].
pub const MM32F52_RAM2_BASE: u32 = 0x3000_0000;

// ---------------------------------------------------------------------------
// Identification
// ---------------------------------------------------------------------------

/// Factory UID of the STM32F1/F0 parts.
pub const F1_UID_BASE: u32 = 0x1fff_f7e8;
/// Factory UID of the STM32F3 parts.
pub const F3_UID_BASE: u32 = 0x1fff_f7ac;

/// Number of option byte halfwords on the STM32/GD32 parts.
pub const STM32F1_OB_COUNT: u16 = 8;
