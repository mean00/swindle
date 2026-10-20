#![allow(dead_code)]

/// FPEC (Flash Program/Erase Controller) register file.
/// Base address: `0x4002_2000` (CH32V003 Reference Manual V1.3, Table 16-2).
pub const FLASH_CONTROLLER_BASE: u32 = 0x4002_2000;

#[repr(C)]
pub struct FlashRegisters {
    pub actlr: u32,         // 0x00 Access control
    pub keyr: u32,          // 0x04 FPEC unlock key
    pub obkeyr: u32,        // 0x08 Option byte key
    pub statr: u32,         // 0x0C Status
    pub ctlr: u32,          // 0x10 Configuration / control
    pub addr: u32,          // 0x14 Address
    pub reserved_18: u32,   // 0x18
    pub obr: u32,           // 0x1C Option byte
    pub wpr: u32,           // 0x20 Write protection
    pub modekeyr: u32,      // 0x24 Extended (fast mode) key
    pub boot_modekeyr: u32, // 0x28 BOOT area key
}

impl FlashRegisters {
    pub const ACTLR_OFFSET: u32 = 0x00;
    pub const KEYR_OFFSET: u32 = 0x04;
    pub const OBKEYR_OFFSET: u32 = 0x08;
    pub const STATR_OFFSET: u32 = 0x0C;
    pub const CTLR_OFFSET: u32 = 0x10;
    pub const ADDR_OFFSET: u32 = 0x14;
    pub const OBR_OFFSET: u32 = 0x1C;
    pub const WPR_OFFSET: u32 = 0x20;
    pub const MODEKEYR_OFFSET: u32 = 0x24;
    pub const BOOT_MODEKEYR_OFFSET: u32 = 0x28;

    #[inline(always)]
    pub const fn reg_addr(offset: u32) -> u32 {
        FLASH_CONTROLLER_BASE + offset
    }
}

// --- FPEC Unlock Keys (16.3.2 / 16.4.2) ---
pub const FLASH_KEY1: u32 = 0x4567_0123;
pub const FLASH_KEY2: u32 = 0xcdef_89ab;

// --- STATR Register Bits (16.3.4) ---
pub const FMC_STAT_BUSY: u32 = 1 << 0;     // Flash operation in progress
pub const FMC_STAT_WRPRTERR: u32 = 1 << 4; // Write protection error
pub const FMC_STAT_EOP: u32 = 1 << 5;      // End of operation

// --- CTLR Register Bits (16.3.5) ---
pub const FMC_CTL_PG: u32 = 1 << 0;       // Standard programming
pub const FMC_CTL_PER: u32 = 1 << 1;      // Standard page erase
pub const FMC_CTL_MER: u32 = 1 << 2;      // Whole chip erase
pub const FMC_CTL_STRT: u32 = 1 << 6;     // Start erase/program (cleared by hardware)
pub const FMC_CTL_LOCK: u32 = 1 << 7;     // FPEC and CTLR locked
pub const FMC_CTL_FLOCK: u32 = 1 << 15;   // Fast programming/erase locked
pub const FMC_CTL_FTPG: u32 = 1 << 16;    // Fast page programming mode
pub const FMC_CTL_FTER: u32 = 1 << 17;    // Fast page erase mode
pub const FMC_CTL_BUFLOAD: u32 = 1 << 18; // Cache the written word into buffer
pub const FMC_CTL_BUFRST: u32 = 1 << 19;  // Reset (clear) internal buffer

// --- System & Device Information Addresses ---
pub const CH32V003X_IDCODE: u32 = 0x1fff_f7c4;
pub const CH32V0X_IDCODE_MASK: u32 = 0x0fff_ff0f;

pub const CH32V0X_ESIG_FLASH_CAP: u32 = 0x1fff_f7e0;
pub const CH32V0X_ESIG_UID1: u32 = 0x1fff_f7e8;
pub const CH32V0X_ESIG_UID2: u32 = 0x1fff_f7ec;
pub const CH32V0X_ESIG_UID3: u32 = 0x1fff_f7f0;

// Known device signatures
pub const IDCODE_CH32V006: u32 = CH32V0X_IDCODE_MASK;
pub const IDCODE_CH32V003_F4P6: u32 = 0x00300500;
pub const IDCODE_CH32V003_F4U6: u32 = 0x00310500;
pub const IDCODE_CH32V003_A4M6: u32 = 0x00320500;
pub const IDCODE_CH32V003_J4M6: u32 = 0x00330500;

// --- Memory Layout Constants ---
pub const RAM_ADDRESS: u32 = 0x2000_0000;
pub const FLASH_ALIAS_ADDRESS: u32 = 0x0000_0000;
pub const FLASH_PHYSICAL_ADDRESS: u32 = 0x0800_0000;
pub const FLASH_BLOCK_SIZE: usize = 512;
pub const FLASH_WORD_BYTES: usize = 4;
pub const FLASH_TIMEOUT_ITERS: usize = 2000;
pub static ERASE_SIZES: [usize; 2] = [FLASH_BLOCK_SIZE, 0];

// Target options
pub const TOPT_INHIBIT_NRST: u32 = 0x4000;

// --- In-SRAM Flashstub Configuration ---
pub const STUB_CODE_LOCATION: u32 = 0x2000_0000;
pub const STUB_DATA_LOCATION: u32 = 0x2000_0200;
pub const STUB_STACK_LOCATION: u32 = 0x2000_0780;
pub const STUB_MAX_CHUNK: usize = 1024;

/// Flashstub binary for high-speed page programming (rv32ec_zicsr / ilp32e).
pub const CH32V0X_WRITE_BIN: &[u8] = &[
    0x2a, 0x84, 0x2a, 0xc2, 0xb2, 0x87, 0x2e, 0x85, 0x73, 0x70, 0x04, 0x30,
    0x37, 0x27, 0x02, 0x40, 0x13, 0xd6, 0x26, 0x00, 0x32, 0xc4, 0x93, 0xf4,
    0xc6, 0xff, 0x31, 0x07, 0x81, 0x43, 0x63, 0xf5, 0xf3, 0x0a, 0x37, 0x26,
    0x02, 0x40, 0xc1, 0x65, 0x0c, 0xca, 0xb7, 0x05, 0x09, 0x00, 0x0c, 0xca,
    0x41, 0x06, 0x10, 0x42, 0x10, 0x43, 0x05, 0x8a, 0x75, 0xfe, 0x10, 0x43,
    0x93, 0x05, 0x00, 0x03, 0x0c, 0xc3, 0x41, 0x8a, 0x3d, 0xee, 0x22, 0x46,
    0x39, 0xc2, 0x2a, 0x86, 0xb7, 0x25, 0x02, 0x40, 0x26, 0x95, 0x33, 0x03,
    0xc4, 0x40, 0xc1, 0x05, 0x1e, 0xc0, 0x83, 0x22, 0x06, 0x00, 0xb3, 0x03,
    0x66, 0x00, 0x23, 0xa0, 0x53, 0x00, 0x0f, 0x00, 0xf0, 0x0f, 0xb7, 0x02,
    0x05, 0x00, 0x23, 0xa0, 0x55, 0x00, 0x83, 0xa3, 0x05, 0x00, 0x83, 0x23,
    0x07, 0x00, 0x93, 0xf3, 0x13, 0x00, 0xe3, 0x9c, 0x03, 0xfe, 0x11, 0x06,
    0xe3, 0x1b, 0xa6, 0xfc, 0x82, 0x43, 0x32, 0x85, 0x26, 0x94, 0x0f, 0x00,
    0xf0, 0x0f, 0x12, 0x46, 0xb7, 0x25, 0x02, 0x40, 0x33, 0x03, 0x76, 0x00,
    0x23, 0xaa, 0x65, 0x00, 0x2e, 0x86, 0xc1, 0x65, 0x93, 0x85, 0x05, 0x04,
    0x0c, 0xca, 0x41, 0x06, 0x10, 0x42, 0x10, 0x43, 0x05, 0x8a, 0x75, 0xfe,
    0x10, 0x43, 0x93, 0x05, 0x00, 0x03, 0x0c, 0xc3, 0x41, 0x8a, 0xb6, 0x93,
    0x39, 0xde, 0x85, 0x47, 0x3e, 0x85, 0x02, 0x90, 0x81, 0x47, 0x3e, 0x85,
    0x02, 0x90,
];
