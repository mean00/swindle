//! Register definitions for Nordic Semiconductor devices (nRF51, nRF52, nRF54L, nRF91).

// --- nRF51 / nRF52 Non-Volatile Memory Controller (NVMC) ---
pub const NRF51_NVMC: u32 = 0x4001_e000;

#[repr(C)]
pub struct NvmcRegisters {
    pub reserved_00: [u32; 256], // 0x000..0x400
    pub ready: u32,              // 0x400
    pub reserved_404: [u32; 64], // 0x404..0x504
    pub config: u32,             // 0x504
    pub erasepage: u32,          // 0x508
    pub eraseall: u32,           // 0x50c
    pub reserved_510: u32,       // 0x510
    pub eraseuicr: u32,          // 0x514
}

const _: () = assert!(::core::mem::offset_of!(NvmcRegisters, ready) == 0x400);
const _: () = assert!(::core::mem::offset_of!(NvmcRegisters, config) == 0x504);
const _: () = assert!(::core::mem::offset_of!(NvmcRegisters, erasepage) == 0x508);
const _: () = assert!(::core::mem::offset_of!(NvmcRegisters, eraseall) == 0x50c);
const _: () = assert!(::core::mem::offset_of!(NvmcRegisters, eraseuicr) == 0x514);

macro_rules! nvmc_reg {
    ($member:ident) => {
        $crate::registers::NRF51_NVMC + ::core::mem::offset_of!($crate::registers::NvmcRegisters, $member) as u32
    };
    ($base:expr, $member:ident) => {
        $base + ::core::mem::offset_of!($crate::registers::NvmcRegisters, $member) as u32
    };
}
pub(crate) use nvmc_reg;

pub const NRF51_NVMC_READY: u32 = nvmc_reg!(ready);
pub const NRF51_NVMC_CONFIG: u32 = nvmc_reg!(config);
pub const NRF51_NVMC_ERASEPAGE: u32 = nvmc_reg!(erasepage);
pub const NRF51_NVMC_ERASEALL: u32 = nvmc_reg!(eraseall);
pub const NRF51_NVMC_ERASEUICR: u32 = nvmc_reg!(eraseuicr);

pub const NRF51_NVMC_CONFIG_REN: u32 = 0x0;
pub const NRF51_NVMC_CONFIG_WEN: u32 = 0x1;
pub const NRF51_NVMC_CONFIG_EEN: u32 = 0x2;

// --- nRF51 / nRF52 Factory Information Configuration Registers (FICR) ---
pub const NRF51_FICR: u32 = 0x1000_0000;

#[repr(C)]
pub struct FicrRegisters {
    pub reserved_00: [u32; 4],   // 0x000..0x010
    pub codepagesize: u32,       // 0x010
    pub codesize: u32,           // 0x014
    pub reserved_18: [u32; 17],  // 0x018..0x05c
    pub configid: u32,           // 0x05c
    pub deviceid_low: u32,       // 0x060
    pub deviceid_high: u32,      // 0x064
    pub reserved_68: [u32; 14],  // 0x068..0x0a0
    pub deviceaddrtype: u32,     // 0x0a0
    pub deviceaddr_low: u32,     // 0x0a4
    pub deviceaddr_high: u32,    // 0x0a8
    pub reserved_ac: [u32; 21],  // 0x0ac..0x100
    pub info_part: u32,          // 0x100
    pub info_variant: u32,       // 0x104
    pub info_package: u32,       // 0x108
    pub info_ram: u32,           // 0x10c
    pub info_flash: u32,         // 0x110
}

const _: () = assert!(::core::mem::offset_of!(FicrRegisters, codepagesize) == 0x010);
const _: () = assert!(::core::mem::offset_of!(FicrRegisters, codesize) == 0x014);
const _: () = assert!(::core::mem::offset_of!(FicrRegisters, configid) == 0x05c);
const _: () = assert!(::core::mem::offset_of!(FicrRegisters, deviceaddrtype) == 0x0a0);
const _: () = assert!(::core::mem::offset_of!(FicrRegisters, info_part) == 0x100);
const _: () = assert!(::core::mem::offset_of!(FicrRegisters, info_ram) == 0x10c);
const _: () = assert!(::core::mem::offset_of!(FicrRegisters, info_flash) == 0x110);

macro_rules! ficr_reg {
    ($member:ident) => {
        $crate::registers::NRF51_FICR + ::core::mem::offset_of!($crate::registers::FicrRegisters, $member) as u32
    };
    ($base:expr, $member:ident) => {
        $base + ::core::mem::offset_of!($crate::registers::FicrRegisters, $member) as u32
    };
}
pub(crate) use ficr_reg;

pub const NRF51_FICR_CODEPAGESIZE: u32 = ficr_reg!(codepagesize);
pub const NRF51_FICR_CODESIZE: u32 = ficr_reg!(codesize);
pub const NRF51_FICR_CONFIGID: u32 = ficr_reg!(configid);
pub const NRF51_FICR_DEVICEID_LOW: u32 = ficr_reg!(deviceid_low);
pub const NRF51_FICR_DEVICEID_HIGH: u32 = ficr_reg!(deviceid_high);
pub const NRF51_FICR_DEVICEADDRTYPE: u32 = ficr_reg!(deviceaddrtype);
pub const NRF51_FICR_DEVICEADDR_LOW: u32 = ficr_reg!(deviceaddr_low);
pub const NRF51_FICR_DEVICEADDR_HIGH: u32 = ficr_reg!(deviceaddr_high);

pub const NRF52_PART_INFO: u32 = ficr_reg!(info_part);
pub const NRF52_INFO_RAM: u32 = ficr_reg!(info_ram);

pub const NRF51_FICR_DEVICE_INFO_BASE: u32 = NRF51_FICR + 0x100;
pub const NRF51_FICR_DEVICE_INFO_PART: u32 = ficr_reg!(info_part);
pub const NRF51_FICR_DEVICE_INFO_VARIANT: u32 = ficr_reg!(info_variant);
pub const NRF51_FICR_DEVICE_INFO_PACKAGE: u32 = ficr_reg!(info_package);
pub const NRF51_FICR_DEVICE_INFO_RAM: u32 = ficr_reg!(info_ram);
pub const NRF51_FICR_DEVICE_INFO_FLASH: u32 = ficr_reg!(info_flash);

pub const NRF51_FIELD_UNSPECIFIED: u32 = 0xffff_ffff;

// --- nRF51 / nRF52 User Information Configuration Registers (UICR) ---
pub const NRF51_UICR: u32 = 0x1000_1000;

#[repr(C)]
pub struct UicrRegisters {
    pub reserved_00: [u32; 130], // 0x000..0x208
    pub approtect: u32,          // 0x208
}

const _: () = assert!(::core::mem::offset_of!(UicrRegisters, approtect) == 0x208);

macro_rules! uicr_reg {
    ($member:ident) => {
        $crate::registers::NRF51_UICR + ::core::mem::offset_of!($crate::registers::UicrRegisters, $member) as u32
    };
}
pub(crate) use uicr_reg;

pub const NRF51_APPROTECT: u32 = uicr_reg!(approtect);

pub const NRF51_PAGE_SIZE: usize = 1024;
pub const NRF52_PAGE_SIZE: usize = 4096;

// --- nRF52 CTRL-AP ---
pub const NRF52_CTRL_AP_IDR: u32 = 0x0288_0000;
pub const CTRL_AP_POWER_EN: u16 = 0x04; // DP reg 1 (CTRL/STAT)
pub const CTRL_AP_SELECT_AP: u16 = 0x08; // DP reg 2 (SELECT)
pub const CTRL_AP_STATUS: u16 = 0x08; // AP reg
pub const CTRL_AP_CONTROL: u16 = 0x04; // AP reg
pub const CTRL_AP_PROT_EN: u16 = 0x0c; // AP reg

// --- nRF54L Series ---
pub const ID_NRF54L: u16 = 0x01c0;

pub const NRF54L_FICR_BASE: u32 = 0x00ff_c000;

#[repr(C)]
pub struct Nrf54lFicrRegisters {
    pub reserved_00: [u32; 202], // 0x000..0x328
    pub info_ram: u32,           // 0x328
    pub info_rram: u32,          // 0x32c
}

const _: () = assert!(::core::mem::offset_of!(Nrf54lFicrRegisters, info_ram) == 0x328);
const _: () = assert!(::core::mem::offset_of!(Nrf54lFicrRegisters, info_rram) == 0x32c);

macro_rules! nrf54l_ficr_reg {
    ($member:ident) => {
        $crate::registers::NRF54L_FICR_BASE + ::core::mem::offset_of!($crate::registers::Nrf54lFicrRegisters, $member) as u32
    };
}
pub(crate) use nrf54l_ficr_reg;

pub const NRF54L_FICR_INFO_RAM: u32 = nrf54l_ficr_reg!(info_ram);
pub const NRF54L_FICR_INFO_RRAM: u32 = nrf54l_ficr_reg!(info_rram);

pub const NRF54L_RRAM: u32 = 0x0000_0000;
pub const NRF54L_UICR: u32 = 0x00ff_d000;
pub const NRF54L_RAM: u32 = 0x2000_0000;

pub const NRF54L_RRAMC_BASE: u32 = 0x5004_b000;

#[repr(C)]
pub struct RramcRegisters {
    pub reserved_00: [u32; 256],     // 0x000..0x400
    pub ready: u32,                   // 0x400
    pub readynext: u32,               // 0x404
    pub reserved_408: [u32; 4],       // 0x408..0x418
    pub bufstatus_writebufempty: u32, // 0x418
    pub reserved_41c: [u32; 57],      // 0x41c..0x500
    pub config: u32,                  // 0x500
    pub reserved_504: [u32; 15],      // 0x504..0x540
    pub erase_eraseall: u32,          // 0x540
}

const _: () = assert!(::core::mem::offset_of!(RramcRegisters, ready) == 0x400);
const _: () = assert!(::core::mem::offset_of!(RramcRegisters, readynext) == 0x404);
const _: () = assert!(::core::mem::offset_of!(RramcRegisters, bufstatus_writebufempty) == 0x418);
const _: () = assert!(::core::mem::offset_of!(RramcRegisters, config) == 0x500);
const _: () = assert!(::core::mem::offset_of!(RramcRegisters, erase_eraseall) == 0x540);

macro_rules! rramc_reg {
    ($member:ident) => {
        $crate::registers::NRF54L_RRAMC_BASE + ::core::mem::offset_of!($crate::registers::RramcRegisters, $member) as u32
    };
}
pub(crate) use rramc_reg;

pub const NRF54L_RRAMC_READY: u32 = rramc_reg!(ready);
pub const NRF54L_RRAMC_READYNEXT: u32 = rramc_reg!(readynext);
pub const NRF54L_RRAMC_BUFSTATUS_WRITEBUFEMPTY: u32 = rramc_reg!(bufstatus_writebufempty);
pub const NRF54L_RRAMC_CONFIG: u32 = rramc_reg!(config);
pub const NRF54L_RRAMC_ERASE_ERASEALL: u32 = rramc_reg!(erase_eraseall);

pub const NRF54L_RRAMC_READY_BUSY: u32 = 0;
pub const NRF54L_RRAMC_READYNEXT_READY: u32 = 1;
pub const NRF54L_RRAMC_BUFSTATUS_WRITEBUFEMPTY_EMPTY: u32 = 1;
pub const NRF54L_RRAMC_CONFIG_WRITE_DISABLED: u32 = 0;
pub const NRF54L_RRAMC_CONFIG_WRITE_ENABLED: u32 = 1;
pub const NRF54L_RRAMC_ERASE_ERASEALL_ERASE: u32 = 1;

pub const NRF54L_CTRL_AP_IDR_VALUE: u32 = 0x3288_0000;
pub const NRF54L_CTRL_AP_RESET: u16 = 0x00;
pub const NRF54L_CTRL_AP_ERASEALL: u16 = 0x04;
pub const NRF54L_CTRL_AP_ERASEALLSTATUS: u16 = 0x08;
pub const NRF54L_CTRL_AP_APPROTECT_STATUS: u16 = 0x14;

pub const NRF54L_CTRL_AP_RESET_NORESET: u32 = 0;
pub const NRF54L_CTRL_AP_RESET_HARDRESET: u32 = 2;
pub const NRF54L_CTRL_AP_ERASEALL_ERASE: u32 = 1;
pub const NRF54L_CTRL_AP_ERASEALLSTATUS_BUSY: u32 = 2;
pub const NRF54L_CTRL_AP_APPROTECT_STATUS_APPROTECT_ENABLED: u32 = 1 << 0;
pub const NRF54L_CTRL_AP_APPROTECT_STATUS_SECUREAPPROTECT_ENABLED: u32 = 1 << 1;

// --- nRF91 Series ---
pub const ID_NRF91: u16 = 0x0090;
pub const NRF91_NVMC: u32 = 0x5003_9000;
pub const NRF91_NVMC_READY: u32 = nvmc_reg!(NRF91_NVMC, ready);
pub const NRF91_NVMC_CONFIG: u32 = nvmc_reg!(NRF91_NVMC, config);
pub const NRF91_NVMC_ERASEALL: u32 = nvmc_reg!(NRF91_NVMC, eraseall);

pub const NRF91_NVMC_CONFIG_REN: u32 = 0x0;
pub const NRF91_NVMC_CONFIG_WEN: u32 = 0x1;
pub const NRF91_NVMC_CONFIG_EEN: u32 = 0x2;
pub const NRF91_NVMC_CONFIG_PEEN: u32 = 0x3;
