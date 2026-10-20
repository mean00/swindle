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

impl FlashRegisters {
    pub const WS_OFFSET: u32 = 0x00;
    pub const KEYR_OFFSET: u32 = 0x04;
    pub const OBKEYR_OFFSET: u32 = 0x08;
    pub const STATR_OFFSET: u32 = 0x0C;
    pub const CTLR_OFFSET: u32 = 0x10;
    pub const ADDR_OFFSET: u32 = 0x14;
    pub const OBR_OFFSET: u32 = 0x1C;
    pub const WPR_OFFSET: u32 = 0x20;
    pub const MODEKEYR_OFFSET: u32 = 0x24;

    #[inline(always)]
    pub const fn reg_addr(offset: u32) -> u32 {
        FLASH_CONTROLLER_BASE + offset
    }
}

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
pub const FMC_CTL_BER64: u32 = 1 << 19;           // Block erase 64 KB
pub const FMC_CTL_FASTSTART: u32 = 1 << 21;       // Start fast programming
pub const FMC_CTL_RSENACT: u32 = 1 << 22;         // RSEN_ACT bit
pub const FMC_CTL_EHMOD: u32 = 1 << 24;           // Enhanced read mode enable
pub const FMC_CTL_SCKMOD: u32 = 1 << 25;          // Flash access clock: 1=SYSCLK, 0=SYSCLK/2

// --- RCC Register Block ---
pub const RCC_BASE: u32 = 0x4002_1000;
pub const RCC_CTLR_OFFSET: u32 = 0x00;
pub const RCC_CFGR0_OFFSET: u32 = 0x04;

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
pub const fn rcc_reg_addr(offset: u32) -> u32 {
    RCC_BASE + offset
}

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
pub const FLASH_BLOCK_SIZE: usize = 4096;
pub const FLASH_WRITE_SIZE: usize = 2048;
pub const FLASH_PAGE_SIZE: usize = 256;
pub static ERASE_SIZES: [usize; 4] = [4096, 32768, 65536, 0];
pub const FLASH_TIMEOUT_ITERS: usize = 50_000;

// --- In-SRAM Flashstub Configuration ---
pub const STUB_STACK_LOCATION: u32 = RAM_ADDRESS + 2 * 1024;        // 0x2000_0800
pub const STUB_STACKEND_LOCATION: u32 = RAM_ADDRESS + 4 * 1024 - 16; // 0x2000_0ff0
pub const STUB_CODE_LOCATION_ERASE: u32 = RAM_ADDRESS + 4 * 1024;   // 0x2000_1000
pub const STUB_CODE_LOCATION_WRITE: u32 = STUB_CODE_LOCATION_ERASE + 512; // 0x2000_1200
pub const STUB_DATA_LOCATION: u32 = RAM_ADDRESS + 6 * 1024;         // 0x2000_1800
pub const ATOMIC_FLASHSTUB_WRITE: usize = 4 * 1024;

/// Flashstub binary for flash erase (64 bytes).
pub const CH32V3X_ERASE_BIN: &[u8] = &[
    0xaa, 0x95, 0x63, 0x7a, 0xb5, 0x02, 0x37, 0x26, 0x02, 0x40, 0xb7, 0x06,
    0x02, 0x00, 0x01, 0x77, 0x7d, 0x17, 0x1c, 0x4a, 0xd5, 0x8f, 0x1c, 0xca,
    0x48, 0xca, 0x1c, 0x4a, 0x93, 0xe7, 0x07, 0x04, 0x1c, 0xca, 0x5c, 0x46,
    0x85, 0x8b, 0xf5, 0xff, 0x1c, 0x4a, 0xf9, 0x8f, 0x13, 0x05, 0x05, 0x10,
    0x1c, 0xca, 0xe3, 0x60, 0xb5, 0xfe, 0x81, 0x45, 0x2e, 0x85, 0x02, 0x90,
    0x05, 0x45, 0x82, 0x80,
];

/// Flashstub binary for flash write (166 bytes).
pub const CH32V3X_WRITE_BIN: &[u8] = &[
    0x37, 0x27, 0x02, 0x40, 0xb3, 0x02, 0xa6, 0x00, 0x41, 0x68, 0x93, 0x03,
    0x00, 0x04, 0xb7, 0x08, 0x20, 0x00, 0x2a, 0x83, 0x63, 0x7a, 0x55, 0x06,
    0x08, 0x4b, 0x33, 0x65, 0x05, 0x01, 0x08, 0xcb, 0x48, 0x47, 0x05, 0x89,
    0x75, 0xfd, 0x01, 0x46, 0x1a, 0x85, 0x03, 0xce, 0x15, 0x00, 0x83, 0xce,
    0x05, 0x00, 0x03, 0xcf, 0x25, 0x00, 0x83, 0xc7, 0x35, 0x00, 0x22, 0x0e,
    0xb3, 0x66, 0xde, 0x01, 0x42, 0x0f, 0xe2, 0x07, 0xb3, 0x67, 0xff, 0x00,
    0xdd, 0x8e, 0x14, 0xc1, 0x54, 0x47, 0x89, 0x8a, 0xf5, 0xfe, 0x91, 0x05,
    0x05, 0x06, 0x11, 0x05, 0xe3, 0x19, 0x76, 0xfc, 0x10, 0x4b, 0x33, 0x66,
    0x16, 0x01, 0x10, 0xcb, 0x50, 0x47, 0x05, 0x8a, 0x75, 0xfe, 0x10, 0x4b,
    0x79, 0x9a, 0x10, 0xcb, 0x50, 0x47, 0x54, 0x47, 0x93, 0x77, 0x86, 0x01,
    0x81, 0xef, 0x93, 0xe6, 0x06, 0x02, 0x13, 0x76, 0x06, 0x02, 0x54, 0xc7,
    0x59, 0xf6, 0x01, 0xa8, 0x81, 0x45, 0x2e, 0x85, 0x02, 0x90, 0x39, 0xa0,
    0x13, 0xe5, 0x86, 0x01, 0x48, 0xc7, 0x85, 0x45, 0x2e, 0x85, 0x02, 0x90,
    0x33, 0x35, 0x53, 0x00, 0x13, 0x45, 0x15, 0x00, 0x82, 0x80,
];
