#![allow(dead_code)]

/// FPEC (Flash Program/Erase Controller) register file for CH32V1xx/V2xx/V3xx.
/// Base address: `0x4002_2000` (CH32FV2x_V3x Reference Manual, Chapter 32.4).
pub const FLASH_CONTROLLER_BASE: u32 = 0x4002_2000;

#[repr(C)]
pub struct FlashRegisters {
    pub ws: u32,        // 0x00 Wait states / access control
    pub keyr: u32,      // 0x04 FPEC key
    pub obkeyr: u32,    // 0x08 Option byte key
    pub statr: u32,     // 0x0C Status
    pub ctlr: u32,      // 0x10 Configuration / control
    pub addr: u32,      // 0x14 Address
    pub filler: u32,    // 0x18
    pub obr: u32,       // 0x1C Option byte
    pub wpr: u32,       // 0x20 Write protection
    pub modekeyr: u32,  // 0x24 Extended (fast mode) key
}

/// Address of a [`FlashRegisters`] member in the target: `FLASH_CONTROLLER_BASE + offsetof(member)`.
///
/// The struct above is the single source of truth: the addresses derive from it
/// (as the legacy C driver did with `offsetof(ch32_flash_s, reg)`). Evaluated at
/// compile time; a misspelled member does not compile.
macro_rules! flash_reg {
    ($member:ident) => {
        FLASH_CONTROLLER_BASE + ::core::mem::offset_of!(FlashRegisters, $member) as u32
    };
}
pub(crate) use flash_reg;

// Datasheet tripwire (CH32FV2x_V3x RM, Chapter 32.4): fails the build if the struct is edited
// so that the last register no longer sits at its documented offset.
const _: () = assert!(::core::mem::offset_of!(FlashRegisters, modekeyr) == 0x24);

// --- FPEC Unlock Keys ---
pub const FLASH_KEY1: u32 = 0x4567_0123;
pub const FLASH_KEY2: u32 = 0xcdef_89ab;

// --- FLASH_STATR Register Bits ---
pub const FMC_STAT_BUSY: u32 = 1 << 0;     // Flash operation in progress
pub const FMC_STAT_WR_BUSY: u32 = 1 << 1;  // Fast programming write buffer busy
pub const FMC_STAT_PG_ERR: u32 = 1 << 3;   // Program error
pub const FMC_STAT_WP_ERR: u32 = 1 << 4;   // Write protection error
pub const FMC_STAT_WP_ENDF: u32 = 1 << 5;  // End of operation flag

// --- FLASH_CTLR Register Bits ---
pub const FMC_CTL_PG: u32 = 1 << 0;               // Standard programming
pub const FMC_CTL_PER: u32 = 1 << 1;              // Standard page erase
pub const FMC_CTL_MER: u32 = 1 << 2;              // Whole chip erase
pub const FMC_CTL_OBPG: u32 = 1 << 4;             // Option byte programming
pub const FMC_CTL_START: u32 = 1 << 6;            // Start erase/programming
pub const FMC_CTL_LK: u32 = 1 << 7;               // Lock bit
pub const FMC_CTL_FASTUNLOCK: u32 = 1 << 15;      // Fast programming lock bit
pub const FMC_CTL_FASTPROGRAM: u32 = 1 << 16;     // Fast 256-byte page programming mode
pub const FMC_CTL_FASTERASE: u32 = 1 << 17;       // Fast 256-byte page erase mode
pub const FMC_CTL_BER32: u32 = 1 << 18;           // Block erase 32 KB
// RM V2.2 (32.4) documents bit 19 as BER64 "Perform a 64KB erase", with its own procedure
// (32.5.7 "Fast Erase erases in blocks (64K bytes)"); the V2.5 text lists [20:19] as
// reserved. Keep the mode: it is the only way to erase 64 KB in a single operation.
pub const FMC_CTL_BER64: u32 = 1 << 19;           // Block erase 64 KB
pub const FMC_CTL_FASTSTART: u32 = 1 << 21;       // Start fast programming
pub const FMC_CTL_RSENACT: u32 = 1 << 22;         // RSEN_ACT bit
pub const FMC_CTL_EHMOD: u32 = 1 << 24;           // Enhanced read mode enable
pub const FMC_CTL_SCKMOD: u32 = 1 << 25;          // Flash access clock: 1=SYSCLK, 0=SYSCLK/2

// --- RCC Register Block ---
pub const RCC_BASE: u32 = 0x4002_1000;

/// RCC register file. Only the registers the driver touches are modelled (the layout
/// stops after `cfgr0`); the rest of the block is deliberately left out.
#[repr(C)]
pub struct RccRegisters {
    pub ctlr: u32,  // 0x00 Control
    pub cfgr0: u32, // 0x04 Clock configuration
}

/// Address of a [`RccRegisters`] member: `RCC_BASE + offsetof(member)`.
macro_rules! rcc_reg {
    ($member:ident) => {
        RCC_BASE + ::core::mem::offset_of!(RccRegisters, $member) as u32
    };
}
pub(crate) use rcc_reg;

// Datasheet tripwire (CH32FV2x_V3x RM, RCC): CFGR0 sits right after CTLR.
const _: () = assert!(::core::mem::offset_of!(RccRegisters, cfgr0) == 0x04);

pub const RCC_CTLR_HSION: u32 = 1 << 0;
pub const RCC_CTLR_HSIRDY: u32 = 1 << 1;
pub const RCC_CTLR_HSEON: u32 = 1 << 16;
pub const RCC_CTLR_HSERDY: u32 = 1 << 17;
pub const RCC_CTLR_PLLON: u32 = 1 << 24;
pub const RCC_CTLR_PLLRDY: u32 = 1 << 25;

pub const RCC_CFGR0_SW_MASK: u32 = 3 << 0;
pub const RCC_CFGR0_SW_HSI: u32 = 0 << 0;
pub const RCC_CFGR0_SW_HSE: u32 = 1 << 0;
pub const RCC_CFGR0_SW_PLL: u32 = 2 << 0;

pub const RCC_CFGR0_SWS_MASK: u32 = 3 << 2;
pub const RCC_CFGR0_SWS_HSI: u32 = 0 << 2;
pub const RCC_CFGR0_SWS_PLL: u32 = 2 << 2;

pub const RCC_CFGR0_HPRE_DIV1: u32 = 0 << 4;
pub const RCC_CFGR0_PPRE1_DIV1: u32 = 0 << 8;
pub const RCC_CFGR0_PPRE2_DIV1: u32 = 0 << 11;
pub const RCC_CFGR0_PLLSRC_HSI: u32 = 0 << 16;
pub const RCC_CFGR0_PLLSRC_HSE: u32 = 1 << 16;

#[inline(always)]
pub const fn rcc_cfg_pllmul(mult: u32) -> u32 {
    (mult & 0x0F) << 18
}

// --- System & Device Information Addresses ---
pub const CHIPID_ADDRESS: u32 = 0x1fff_f704;
pub const OPTION_BYTES_ADDRESS: u32 = 0x1fff_f800;
pub const UID1_ADDRESS: u32 = 0x1fff_f7e8;

pub const CHIPID_FAMILY_OFFSET: u32 = 20;
pub const CHIPID_FAMILY_MASK: u32 = 0xFFF << CHIPID_FAMILY_OFFSET;

pub const FAMILY_CH32V103: u16 = 0x103;
pub const FAMILY_CH32V203: u16 = 0x203;
pub const FAMILY_CH32V208: u16 = 0x208;
pub const FAMILY_CH32V303: u16 = 0x303;
pub const FAMILY_CH32V305: u16 = 0x305;
pub const FAMILY_CH32V307: u16 = 0x307;

// --- Memory Layout Constants ---
pub const RAM_ADDRESS: u32 = 0x2000_0000;
pub const FLASH_ALIAS_ADDRESS: u32 = 0x0000_0000;
pub const FLASH_PHYSICAL_ADDRESS: u32 = 0x0800_0000;
/// Programming page: one half-word PG burst writes at most this many bytes.
pub const FLASH_PAGE_SIZE: usize = 256;
/// Bytes the flash manager may send in one `vFlashWrite` packet.
pub const FLASH_WRITE_SIZE: usize = 2048;

// --- Erase block sizes advertised to the flash manager (`FlashDesc::erasesizes`) ---
//
// These are the only erase sizes GDB is allowed to ask for. They are NOT necessarily the
// controller's physical erase page: the driver maps each advertised size onto an erase
// mode, and when that mode erases less than the advertised size it iterates the physical
// pages itself:
//   ERASE_BLOCK_SIZE_4K  -> in-SRAM stub: 4 KB as 16 x 256 B fast page erase (FTER)
//   ERASE_BLOCK_SIZE_32K -> one BER32 fast block erase
//   ERASE_BLOCK_SIZE_64K -> one BER64 fast block erase
pub const ERASE_BLOCK_SIZE_4K: usize = 4 * 1024;
pub const ERASE_BLOCK_SIZE_32K: usize = 32 * 1024;
pub const ERASE_BLOCK_SIZE_64K: usize = 64 * 1024;
/// Erase sizes offered to the flash manager, ascending, 0 terminated.
///
/// Note: after a fast erase (FTER/BER32/BER64) an erased word reads `0xE339E339`, not
/// `0xFF` (RM 32.5.7 note), so erase verification must expect that value.
pub static ERASE_SIZES: [usize; 4] = [
    ERASE_BLOCK_SIZE_4K,
    ERASE_BLOCK_SIZE_32K,
    ERASE_BLOCK_SIZE_64K,
    0,
];
pub const FLASH_TIMEOUT_ITERS: usize = 50_000;

// --- In-SRAM Flashstub Configuration ---
pub const STUB_STACK_LOCATION: u32 = RAM_ADDRESS + 2 * 1024;        // 0x2000_0800
pub const STUB_STACKEND_LOCATION: u32 = RAM_ADDRESS + 4 * 1024 - 16; // 0x2000_0ff0
pub const STUB_CODE_LOCATION_ERASE: u32 = RAM_ADDRESS + 4 * 1024;   // 0x2000_1000
pub const STUB_CODE_LOCATION_WRITE: u32 = STUB_CODE_LOCATION_ERASE + 512; // 0x2000_1200
pub const STUB_CODE_LOCATION_CRC: u32 = RAM_ADDRESS + 5 * 1024;     // 0x2000_1400
pub const STUB_DATA_LOCATION: u32 = RAM_ADDRESS + 6 * 1024;         // 0x2000_1800
pub const ATOMIC_FLASHSTUB_WRITE: usize = 4 * 1024;

/// SRAM window the driver borrows for stub code, stub stack and stub data.
///
/// A CRC that covered part of it would include the CRC stub's own bytes (the stub runs
/// from there), so such a range is left to the software CRC.
pub const STUB_WORKSPACE_START: u32 = STUB_STACK_LOCATION;                        // 0x2000_0800
pub const STUB_WORKSPACE_END: u32 = STUB_DATA_LOCATION + ATOMIC_FLASHSTUB_WRITE as u32; // 0x2000_2800

// The stub images themselves live in `flashstub.rs`, generated by the
// `flashstub` CMake project (target `flashstub_rs`).
