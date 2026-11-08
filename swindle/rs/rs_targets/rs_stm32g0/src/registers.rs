/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (c) 2021-2023 Fabrice Prost-Boucle <fabalthazar@falbalab.fr>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the register map of blackmagic/src/target/stm32g0.c (FPEC, RCC, DBGMCU,
 * OTP and option byte registers), made for the Swindle firmware. This file is derived
 * from that C source and stays under the same license as the original Black Magic code
 * reproduced in `lib.rs`.
 */

//! Register map of the STM32G0/C0/U0 flash controller (FPEC), of the debug MCU block
//! and of the option byte block.
//!
//! References (same as the C driver):
//! RM0454 (STM32G0x0), RM0444 (STM32G0x1), RM0490 (STM32C0x1), RM0503 (STM32U0).

pub use rs_stm32_common::registers::*;

// ---------------------------------------------------------------------------
// Flash and OTP
// ---------------------------------------------------------------------------

/// Address of the flash size register of the G0B/C, C0 and U0x3 parts, in KiB.
pub const FLASH_SIZE_REG: u32 = 0x1fff_75e0;
/// Flash size register of the STM32U031 (RM0503 §38.2 p1314).
pub const U031_FLASH_SIZE_REG: u32 = 0x1fff_3ea0;
/// Flash size register of the STM32U073/083.
pub const U0X3_FLASH_SIZE_REG: u32 = 0x1fff_6ea0;

/// Base address of the OTP area, mapped as a second flash region.
pub const OTP_BASE: u32 = 0x1fff_7000;
/// Length of the OTP area.
pub const OTP_SIZE: usize = 0x400;
/// Program step of the OTP area: it is written 8 bytes at a time.
pub const OTP_BLOCKSIZE: usize = 0x8;

/// Erase page size of the main flash.
pub const FLASH_PAGE_SIZE: usize = 0x800;
/// First page number of the second flash bank.
pub const FLASH_BANK2_START_PAGE: u32 = 256;

// ---------------------------------------------------------------------------
// FPEC (flash interface)
// ---------------------------------------------------------------------------

/// Base of the flash controller of the G0/C0/U0 parts.
pub const FPEC_BASE: u32 = 0x4002_2000;

#[repr(C)]
pub struct FlashRegisters {
    pub acr: u32,              // 0x00 Access control
    pub reserved_04: u32,      // 0x04
    pub keyr: u32,             // 0x08 FPEC key
    pub optkeyr: u32,          // 0x0C Option byte key
    pub sr: u32,               // 0x10 Status
    pub cr: u32,               // 0x14 Control
    pub reserved_18: u32,      // 0x18
    pub reserved_1c: u32,      // 0x1C
    pub optr: u32,             // 0x20 Option register
    pub pcrop1asr: u32,        // 0x24 PCROP1A start
    pub pcrop1aer: u32,        // 0x28 PCROP1A end
    pub wrp1ar: u32,           // 0x2C WRP1A
    pub wrp1br: u32,           // 0x30 WRP1B
    pub pcrop1bsr: u32,        // 0x34 PCROP1B start
    pub pcrop1ber: u32,        // 0x38 PCROP1B end
    pub reserved_3c: u32,      // 0x3C
    pub reserved_40: u32,      // 0x40
    pub pcrop2asr: u32,        // 0x44 PCROP2A start
    pub pcrop2aer: u32,        // 0x48 PCROP2A end
    pub wrp2ar: u32,           // 0x4C WRP2A
    pub wrp2br: u32,           // 0x50 WRP2B
    pub pcrop2bsr: u32,        // 0x54 PCROP2B start
    pub pcrop2ber: u32,        // 0x58 PCROP2B end
    pub reserved_5c: [u32; 9], // 0x5C..0x80
    pub secr: u32,             // 0x80 Security option
}

// Datasheet tripwire
const _: () = assert!(::core::mem::offset_of!(FlashRegisters, secr) == 0x80);

/// Address of a [`FlashRegisters`] member in the target: `FPEC_BASE + offsetof(member)`.
macro_rules! flash_reg {
    ($member:ident) => {
        $crate::registers::FPEC_BASE + ::core::mem::offset_of!($crate::registers::FlashRegisters, $member) as u32
    };
}
pub(crate) use flash_reg;

pub const FPEC_ACCESS_CTRL: u32 = flash_reg!(acr);
pub const FPEC_KEY: u32 = flash_reg!(keyr);
pub const FPEC_OPTION_KEY: u32 = flash_reg!(optkeyr);
pub const FPEC_STATUS: u32 = flash_reg!(sr);
pub const FPEC_CTRL: u32 = flash_reg!(cr);
pub const FPEC_OPTION: u32 = flash_reg!(optr);
pub const FPEC_PCROP1A_START: u32 = flash_reg!(pcrop1asr);
pub const FPEC_PCROP1A_END: u32 = flash_reg!(pcrop1aer);
pub const FPEC_WRP1A: u32 = flash_reg!(wrp1ar);
pub const FPEC_WRP1B: u32 = flash_reg!(wrp1br);
pub const FPEC_PCROP1B_START: u32 = flash_reg!(pcrop1bsr);
pub const FPEC_PCROP1B_END: u32 = flash_reg!(pcrop1ber);
pub const FPEC_PCROP2A_START: u32 = flash_reg!(pcrop2asr);
pub const FPEC_PCROP2A_END: u32 = flash_reg!(pcrop2aer);
pub const FPEC_WRP2A: u32 = flash_reg!(wrp2ar);
pub const FPEC_WRP2B: u32 = flash_reg!(wrp2br);
pub const FPEC_PCROP2B_START: u32 = flash_reg!(pcrop2bsr);
pub const FPEC_PCROP2B_END: u32 = flash_reg!(pcrop2ber);
pub const FPEC_SECURITY_OPT: u32 = flash_reg!(secr);

/// `ACR`: the main flash is empty.
pub const FPEC_ACCESS_CTRL_EMPTY: u32 = 1 << 16;
/// `CR`: the controller is locked.
pub const FPEC_CTRL_LOCK: u32 = 1 << 31;
/// `CR`: launch the option byte loading (system reset follows).
pub const FPEC_CTRL_OBL_LAUNCH: u32 = 1 << 27;
/// `CR`: start programming the option registers.
pub const FPEC_CTRL_OPTSTART: u32 = 1 << 17;
/// `CR`: start the operation written in the other bits.
pub const FPEC_CTRL_START: u32 = 1 << 16;
/// `CR`: mass erase of the second bank.
pub const FPEC_CTRL_MER2: u32 = 1 << 15;
/// `CR`: the erase applies to the second bank (`PNB` then counts in bank 2).
pub const FPEC_CTRL_BKER: u32 = 1 << 13;
/// `CR`: mass erase of the first bank.
pub const FPEC_CTRL_MER1: u32 = 1 << 2;
/// `CR`: shift of the page number field (`PNB`) of a page erase.
pub const FPEC_CTRL_PNB_SHIFT: u32 = 3;
/// `CR`: page erase.
pub const FPEC_CTRL_PER: u32 = 1 << 1;
/// `CR`: flash program.
pub const FPEC_CTRL_PG: u32 = 1 << 0;

/// `SR`: bank 2 is busy.
pub const FPEC_STATUS_BSY2: u32 = 1 << 17;
/// `SR`: bank 1 is busy.
pub const FPEC_STATUS_BSY1: u32 = 1 << 16;
/// `SR`: an option byte pair is invalid.
pub const FPEC_STATUS_OPTVERR: u32 = 1 << 15;
/// `SR`: read protection error.
pub const FPEC_STATUS_RDERR: u32 = 1 << 14;
/// `SR`: the fast programming sequence was interrupted.
pub const FPEC_STATUS_FASTERR: u32 = 1 << 9;
/// `SR`: a write was attempted outside the programmed area.
pub const FPEC_STATUS_MISSERR: u32 = 1 << 8;
/// `SR`: programming sequence error.
pub const FPEC_STATUS_PGSERR: u32 = 1 << 7;
/// `SR`: wrong data size.
pub const FPEC_STATUS_SIZERR: u32 = 1 << 6;
/// `SR`: alignment error.
pub const FPEC_STATUS_PGAERR: u32 = 1 << 5;
/// `SR`: write to a protected area.
pub const FPEC_STATUS_WRPERR: u32 = 1 << 4;
/// `SR`: program error.
pub const FPEC_STATUS_PROGERR: u32 = 1 << 3;
/// `SR`: erase/program operation error.
pub const FPEC_STATUS_OPERR: u32 = 1 << 1;
/// `SR`: end of operation.
pub const FPEC_STATUS_EOP: u32 = 1 << 0;

/// Any of the `SR` error flags.
pub const FPEC_STATUS_ERROR_MASK: u32 = FPEC_STATUS_OPTVERR
    | FPEC_STATUS_RDERR
    | FPEC_STATUS_FASTERR
    | FPEC_STATUS_MISSERR
    | FPEC_STATUS_PGSERR
    | FPEC_STATUS_SIZERR
    | FPEC_STATUS_PGAERR
    | FPEC_STATUS_WRPERR
    | FPEC_STATUS_PROGERR
    | FPEC_STATUS_OPERR;
/// Either bank still running an operation.
pub const FPEC_STATUS_BSY_MASK: u32 = FPEC_STATUS_BSY2 | FPEC_STATUS_BSY1;

/// First option byte unlock key.
pub const FPEC_OPTION_KEY1: u32 = 0x0819_2a3b;
/// Second option byte unlock key.
pub const FPEC_OPTION_KEY2: u32 = 0x4c5d_6e7f;
/// `OPTR` field carrying the read protection level.
pub const FPEC_OPTION_RDP_MASK: u32 = 0xff;
/// Read protection level 2: irreversible, only applied on request.
pub const FPEC_OPTION_RDP_LEVEL2: u32 = 0xcc;
/// Default `OPTR` of the G0x1 parts.
pub const OPTION_DEFAULT_G0X1: u32 = 0xffff_feaa;
/// Default `OPTR` of the C0x1 parts (`BOREN` is set there, RM0490 §4.4.1 p66).
pub const OPTION_DEFAULT_C0X1: u32 = 0x3fff_ffaa;

// ---------------------------------------------------------------------------
// RCC and DBGMCU
// ---------------------------------------------------------------------------

pub const RCC_BASE: u32 = 0x4002_1000;

#[repr(C)]
pub struct RccRegisters {
    pub reserved_00: [u32; 15], // 0x00..0x3C
    pub apbenr1: u32,            // 0x3C
}

const _: () = assert!(::core::mem::offset_of!(RccRegisters, apbenr1) == 0x3c);

macro_rules! rcc_reg {
    ($member:ident) => {
        $crate::registers::RCC_BASE + ::core::mem::offset_of!($crate::registers::RccRegisters, $member) as u32
    };
}
pub(crate) use rcc_reg;

/// `RCC_APBENR1`: enable for the DBGMCU clock.
pub const RCC_APBENR1: u32 = rcc_reg!(apbenr1);
pub const RCC_APBENR1_DBGEN: u32 = 1 << 27;

pub const DBGMCU_BASE: u32 = 0x4001_5800;

#[repr(C)]
pub struct DbgmcuRegisters {
    pub idcode: u32,     // 0x00
    pub cr: u32,         // 0x04
    pub apb1freeze: u32, // 0x08
}

const _: () = assert!(::core::mem::offset_of!(DbgmcuRegisters, apb1freeze) == 0x08);

macro_rules! dbgmcu_reg {
    ($member:ident) => {
        $crate::registers::DBGMCU_BASE + ::core::mem::offset_of!($crate::registers::DbgmcuRegisters, $member) as u32
    };
}
pub(crate) use dbgmcu_reg;

pub const DBGMCU_IDCODE: u32 = dbgmcu_reg!(idcode);
pub const DBGMCU_CONFIG: u32 = dbgmcu_reg!(cr);
pub const DBGMCU_APBFREEZE1: u32 = dbgmcu_reg!(apb1freeze);

pub const DBGMCU_CONFIG_STOP: u32 = 1 << 1;
pub const DBGMCU_CONFIG_STANDBY: u32 = 1 << 2;
pub const DBGMCU_APBFREEZE1_WWDG: u32 = 1 << 11;
pub const DBGMCU_APBFREEZE1_IWDG: u32 = 1 << 12;

// ---------------------------------------------------------------------------
// Identification
// ---------------------------------------------------------------------------

/// Factory UID of the STM32C0 parts.
pub const C0_UID_BASE: u32 = 0x1fff_7550;
/// Factory UID of the STM32G0 parts.
pub const G0_UID_BASE: u32 = 0x1fff_7590;
/// Factory UID of the STM32U031 (RM0503 §33.1 p1313).
pub const U031_UID_BASE: u32 = 0x1fff_6e50;
/// Factory UID of the STM32U073/083.
pub const U0X3_UID_BASE: u32 = 0x1fff_3e50;

// ---------------------------------------------------------------------------
// Option registers
// ---------------------------------------------------------------------------

/// One option register and the value `mon option erase` resets it to.
#[derive(Debug, Clone, Copy)]
pub struct OptionRegister {
    pub addr: u32,
    pub value: u32,
}

/// Number of option registers of the G0/C0/U0 parts (C `OPT_REG_COUNT`).
pub const OPTION_REGISTER_COUNT: usize = 14;

// Indices into [`OPTION_REGISTERS`], in the C `stm32g0_option_bytes_registers_e` order.
pub const OPT_REG_OPTR: usize = 0;
pub const OPT_REG_PCROP1ASR: usize = 1;
pub const OPT_REG_PCROP1AER: usize = 2;
pub const OPT_REG_WRP1AR: usize = 3;
pub const OPT_REG_WRP1BR: usize = 4;
pub const OPT_REG_PCROP1BSR: usize = 5;
pub const OPT_REG_PCROP1BER: usize = 6;
pub const OPT_REG_PCROP2ASR: usize = 7;
pub const OPT_REG_PCROP2AER: usize = 8;
pub const OPT_REG_WRP2AR: usize = 9;
pub const OPT_REG_WRP2BR: usize = 10;
pub const OPT_REG_PCROP2BSR: usize = 11;
pub const OPT_REG_PCROP2BER: usize = 12;
pub const OPT_REG_SECR: usize = 13;

/// Defaults of the option registers (C `stm32g0_options_def`).
///
/// The G0x1 defaults are safe on the G0x0 parts (`IRHEN`/`BOREN` are reserved there),
/// but not on the C0x1 parts, whose `BOREN` is live: that is why `mon option erase`
/// swaps [`OptionRegister::value`] of [`OPT_REG_OPTR`] for [`OPTION_DEFAULT_C0X1`] on
/// the C0 parts.
pub const OPTION_REGISTERS: [OptionRegister; OPTION_REGISTER_COUNT] = [
    OptionRegister { addr: FPEC_OPTION, value: OPTION_DEFAULT_G0X1 },
    OptionRegister { addr: FPEC_PCROP1A_START, value: 0xffff_ffff },
    OptionRegister { addr: FPEC_PCROP1A_END, value: 0x0000_0000 },
    OptionRegister { addr: FPEC_WRP1A, value: 0x0000_00ff },
    OptionRegister { addr: FPEC_WRP1B, value: 0x0000_00ff },
    OptionRegister { addr: FPEC_PCROP1B_START, value: 0xffff_ffff },
    OptionRegister { addr: FPEC_PCROP1B_END, value: 0x0000_0000 },
    OptionRegister { addr: FPEC_PCROP2A_START, value: 0xffff_ffff },
    OptionRegister { addr: FPEC_PCROP2A_END, value: 0x0000_0000 },
    OptionRegister { addr: FPEC_WRP2A, value: 0x0000_00ff },
    OptionRegister { addr: FPEC_WRP2B, value: 0x0000_00ff },
    OptionRegister { addr: FPEC_PCROP2B_START, value: 0xffff_ffff },
    OptionRegister { addr: FPEC_PCROP2B_END, value: 0x0000_0000 },
    OptionRegister { addr: FPEC_SECURITY_OPT, value: 0x0000_0000 },
];
