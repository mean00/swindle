/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2017, 2018 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the register maps of blackmagic/src/target/stm32f4.c
 * (FPEC and debug MCU blocks), made for the Swindle firmware.
 */

//! Register map of the STM32F4-style flash controller (FPEC), DBGMCU block,
//! and related constants for STM32F2/F4/F7 and GD32F4 parts.

#![allow(unused_imports)]

pub use rs_stm32_common::registers::*;

// ---------------------------------------------------------------------------
// FPEC of the STM32F2/F4/F7 and GD32F4 parts
// ---------------------------------------------------------------------------

/// Base of the flash controller of the F4-style parts.
pub const F4_FPEC_BASE: u32 = 0x4002_3c00;

#[repr(C)]
pub struct FlashRegisters {
    pub acr: u32,       // 0x00 Access control
    pub keyr: u32,      // 0x04 Key
    pub optkeyr: u32,   // 0x08 Option key
    pub sr: u32,        // 0x0C Status
    pub cr: u32,        // 0x10 Control
    pub optcr: u32,     // 0x14 Option control
    pub optcr1: u32,    // 0x18 Option control 1
    pub optcr2: u32,    // 0x1C Option control 2 (F72x)
}

const _: () = assert!(::core::mem::offset_of!(FlashRegisters, optcr2) == 0x1c);

/// Address of a [`FlashRegisters`] member in the target: `F4_FPEC_BASE + offsetof(member)`.
macro_rules! flash_reg {
    ($member:ident) => {
        $crate::registers::F4_FPEC_BASE + ::core::mem::offset_of!($crate::registers::FlashRegisters, $member) as u32
    };
}
pub(crate) use flash_reg;

pub const F4_FPEC_ACCESS_CTRL: u32 = flash_reg!(acr);
pub const F4_FPEC_KEY: u32 = flash_reg!(keyr);
pub const F4_FPEC_OPTION_KEY: u32 = flash_reg!(optkeyr);
pub const F4_FPEC_STATUS: u32 = flash_reg!(sr);
pub const F4_FPEC_CTRL: u32 = flash_reg!(cr);
pub const F4_FPEC_OPTION_CTRL: u32 = flash_reg!(optcr);
pub const F4_FPEC_OPTION_CTRL1: u32 = flash_reg!(optcr1);
pub const F4_FPEC_OPTION_CTRL2: u32 = flash_reg!(optcr2);

/// First FPEC unlock key of the F4-style controller.
pub const F4_FPEC_KEY1: u32 = 0x4567_0123;
/// Second FPEC unlock key of the F4-style controller.
pub const F4_FPEC_KEY2: u32 = 0xcdef_89ab;
/// First option byte unlock key of the F4-style controller.
pub const F4_FPEC_OPTION_KEY1: u32 = 0x0819_2a3b;
/// Second option byte unlock key of the F4-style controller.
pub const F4_FPEC_OPTION_KEY2: u32 = 0x4c5d_6e7f;

/// `SR`: an operation is running (bit 16 here, bit 0 on the F1).
pub const F4_FPEC_STATUS_BUSY: u32 = 1 << 16;
/// `SR`: end of operation.
pub const F4_FPEC_STATUS_EOP: u32 = 1 << 0;
/// `SR`: any error flag.
pub const F4_FPEC_STATUS_ERROR_MASK: u32 = 0xf2;

/// `CR`: flash program.
pub const F4_FPEC_CTRL_PG: u32 = 1 << 0;
/// `CR`: sector erase.
pub const F4_FPEC_CTRL_SER: u32 = 1 << 1;
/// `CR`: mass erase.
pub const F4_FPEC_CTRL_MER: u32 = 1 << 2;
/// `CR`: sector number field (`SNB`) shift and mask; the erase names a sector.
pub const F4_FPEC_CTRL_SNB_SHIFT: u32 = 3;
pub const F4_FPEC_CTRL_SNB_MASK: u32 = 0xf << F4_FPEC_CTRL_SNB_SHIFT;
/// `CR`: program parallelism (`PSIZE`) shift and mask.
pub const F4_FPEC_CTRL_PSIZE_SHIFT: u32 = 8;
pub const F4_FPEC_CTRL_PSIZE_MASK: u32 = 0x3 << F4_FPEC_CTRL_PSIZE_SHIFT;
/// `CR`: mass erase of the second bank as well.
pub const F4_FPEC_CTRL_MER1: u32 = 1 << 15;
/// `CR`: start an erase / program operation.
pub const F4_FPEC_CTRL_STRT: u32 = 1 << 16;
/// `CR`: end-of-operation interrupt enable.
pub const F4_FPEC_CTRL_EOPIE: u32 = 1 << 24;
/// `CR`: error interrupt enable.
pub const F4_FPEC_CTRL_ERRIE: u32 = 1 << 25;
/// `CR`: the controller is locked.
pub const F4_FPEC_CTRL_LOCK: u32 = 1 << 31;

/// `OPTCR`: option byte writes are locked.
pub const F4_FPEC_OPTION_CTRL_OPTLOCK: u32 = 1 << 0;
/// `OPTCR`: start an option byte program / erase operation.
pub const F4_FPEC_OPTION_CTRL_OPTSTRT: u32 = 1 << 1;
/// `OPTCR`: the independent watchdog runs from the software clock.
pub const F4_FPEC_OPTION_CTRL_WDG_SW: u32 = 1 << 5;
/// `OPTCR`: read protection level field.
pub const F4_FPEC_OPTION_CTRL_PROT_SHIFT: u32 = 8;
pub const F4_FPEC_OPTION_CTRL_PROT_MASK: u32 = 0xff << F4_FPEC_OPTION_CTRL_PROT_SHIFT;
/// Read protection level 0 (unprotected).
pub const F4_FPEC_OPTION_CTRL_PROT_L0: u32 = 0xaa << F4_FPEC_OPTION_CTRL_PROT_SHIFT;
/// Read protection level 1.
pub const F4_FPEC_OPTION_CTRL_PROT_L1: u32 = 0xbb << F4_FPEC_OPTION_CTRL_PROT_SHIFT;
/// `OPTCR`: the F7 part is in single-bank mode.
pub const F4_FPEC_OPTION_CTRL_NDBANK: u32 = 1 << 29;
/// `OPTCR`: a 1 MiB F42x/F46x part is in dual-bank mode.
pub const F4_FPEC_OPTION_CTRL_DB1M: u32 = 1 << 30;

// ---------------------------------------------------------------------------
// DBGMCU
// ---------------------------------------------------------------------------

/// `DBGMCU` of the STM32F2/F4/F7 and GD32F4 parts.
pub const F4_DBGMCU_BASE: u32 = 0xe004_2000;

#[repr(C)]
pub struct DbgmcuRegisters {
    pub idcode: u32,      // 0x00
    pub cr: u32,          // 0x04
    pub apb1freeze: u32,  // 0x08
}

const _: () = assert!(::core::mem::offset_of!(DbgmcuRegisters, apb1freeze) == 0x08);

macro_rules! dbgmcu_reg {
    ($member:ident) => {
        $crate::registers::F4_DBGMCU_BASE + ::core::mem::offset_of!($crate::registers::DbgmcuRegisters, $member) as u32
    };
}
pub(crate) use dbgmcu_reg;

pub const F4_DBGMCU_IDCODE: u32 = dbgmcu_reg!(idcode);
pub const F4_DBGMCU_CTRL: u32 = dbgmcu_reg!(cr);
pub const F4_DBGMCU_APB1FREEZE: u32 = dbgmcu_reg!(apb1freeze);

pub const F4_DBGMCU_CTRL_DBG_SLEEP: u32 = 1 << 0;
pub const F4_DBGMCU_CTRL_DBG_STOP: u32 = 1 << 1;
pub const F4_DBGMCU_CTRL_DBG_STANDBY: u32 = 1 << 2;
pub const F4_DBGMCU_APB1FREEZE_WWDG: u32 = 1 << 11;
pub const F4_DBGMCU_APB1FREEZE_IWDG: u32 = 1 << 12;

// ---------------------------------------------------------------------------
// Identification & Memory
// ---------------------------------------------------------------------------

/// Factory UID of the STM32F2/F4 and GD32F4 parts.
pub const F4_UID_BASE: u32 = 0x1fff_7a10;
/// Factory UID of the STM32F7 parts.
pub const F7_UID_BASE: u32 = 0x1ff0_f420;
/// Factory UID of the STM32F72x/F73x parts.
pub const F72X_UID_BASE: u32 = 0x1ff0_7a10;

/// ITCM RAM of the STM32F7 parts.
pub const F7_ITCM_RAM_BASE: u32 = 0x0000_0000;
pub const F7_ITCM_RAM_LEN: usize = 0x4000;
/// ITCM alias of the flash on the STM32F7 parts (cached, instruction fetch only).
pub const F7_ITCM_FLASH_BASE: u32 = 0x0020_0000;
/// Core-coupled RAM of the STM32F4 parts and the GD32F4 clones.
pub const F4_CCM_RAM_BASE: u32 = 0x1000_0000;
pub const F4_CCM_RAM_LEN: usize = 0x1_0000;

// Part ids of the F4-style families, as reported by the DBGMCU `IDCODE`.

pub const PART_STM32F20X: u16 = 0x411;
pub const PART_STM32F40X: u16 = 0x413;
pub const PART_STM32F42X: u16 = 0x419;
pub const PART_STM32F446: u16 = 0x421;
pub const PART_STM32F401C: u16 = 0x423;
pub const PART_STM32F411: u16 = 0x431;
pub const PART_STM32F401E: u16 = 0x433;
pub const PART_STM32F46X: u16 = 0x434;
pub const PART_STM32F412: u16 = 0x441;
pub const PART_STM32F74X: u16 = 0x449;
pub const PART_STM32F76X: u16 = 0x451;
pub const PART_STM32F72X: u16 = 0x452;
pub const PART_STM32F410: u16 = 0x458;
pub const PART_STM32F413: u16 = 0x463;
pub const PART_GD32F450: u16 = 0x2b3;
pub const PART_GD32F470: u16 = 0xa2e;
pub const PART_GD32F405: u16 = 0xfa4;
