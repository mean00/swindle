//! Register map of the RP2040 that the flash driver touches.
//!
//! Port of the `#define`s at the top of `blackmagic/src/target/rp2040.c`.

// `x << 0` is kept on purpose: the value then mirrors its bit position in the
// datasheet field it belongs to (e.g. SSI_XIP_SPI_CTRL0_*).
#![allow(clippy::identity_op)]

/// Part number the Cortex-M probe reports for an RP2040 (C `ID_RP2040`).
pub const PART_ID: u32 = 0x1002;

// --- Boot ROM -----------------------------------------------------------------

/// Where the boot ROM stamps its magic and version (C `RP_BOOTROM_MAGIC_ADDR`).
pub const BOOTROM_MAGIC_ADDR: u32 = 0x0000_0010;
/// The `"Mu"` magic plus a version byte in bits 16..24 (C `RP_BOOTROM_MAGIC`).
pub const BOOTROM_MAGIC: u32 = b'M' as u32 | ((b'u' as u32) << 8) | (1 << 16);
/// Everything but the version byte (C `RP_BOOTROM_MAGIC_MASK`).
pub const BOOTROM_MAGIC_MASK: u32 = 0x00ff_ffff;
/// Shift of the version byte (C `RP_BOOTROM_VERSION_SHIFT`).
pub const BOOTROM_VERSION_SHIFT: u32 = 24;
/// Pointer to the ROM function table, 16-bit offset from the ROM base
/// (C `RP_BOOTROM_FUNC_TABLE_ADDR`).
pub const BOOTROM_FUNC_TABLE_ADDR: u32 = 0x0000_0014;
/// Tag of the "reset into USB boot" entry, `'U'` then `'B'` (C
/// `RP_BOOTROM_FUNC_TABLE_TAG('U', 'B')`).
pub const BOOTROM_FUNC_TABLE_TAG_UB: u16 = b'U' as u16 | ((b'B' as u16) << 8);
/// Longest table the probe reads before giving up (C `RP_MAX_TABLE_SIZE`).
pub const MAX_TABLE_SIZE: usize = 0x80;

// --- Memory map ---------------------------------------------------------------

/// Where the QSPI flash is mapped (C `RP_XIP_FLASH_BASE`).
pub const XIP_FLASH_BASE: u32 = 0x1000_0000;
/// Start of the 264 KiB SRAM (C `RP_SRAM_BASE`).
pub const SRAM_BASE: u32 = 0x2000_0000;
/// Size of the SRAM (C `RP_SRAM_SIZE`).
pub const SRAM_SIZE: usize = 0x0004_2000;
/// Where the flash write stub gets its data from (C `RP_STUB_BUFFER_BASE`).
pub const STUB_BUFFER_BASE: u32 = SRAM_BASE + 0x1000;
/// Largest flash the RP2040 can have behind the QSPI pads (C `RP2040_MAX_FLASH`).
pub const MAX_FLASH: usize = 16 * 1024 * 1024;

/// Address the CRC stub is loaded at (C `RP_CRC_STUB_ADDR`, which C leaves at the base of
/// SRAM; Swindle moves it one KiB up).
///
/// The three SRAM windows the driver borrows have to stay disjoint: the flash write stub
/// image at [`SRAM_BASE`], the CRC stub window here, and the write stub's data buffer at
/// [`STUB_BUFFER_BASE`]. The CRC drivers and `flash::write` assert the arithmetic.
pub const CRC_STUB_ADDR: u32 = SRAM_BASE + 0x400;

// --- Register access modes (C `RP_REG_ACCESS_*`) -------------------------------

/// Plain read/write.
pub const REG_ACCESS_NORMAL: u32 = 0x0000;
/// Atomic bit set on write.
pub const REG_ACCESS_WRITE_ATOMIC_BITSET: u32 = 0x2000;
/// Atomic bit clear on write.
pub const REG_ACCESS_WRITE_ATOMIC_BITCLR: u32 = 0x3000;

// --- CLOCKS -------------------------------------------------------------------

pub const CLOCKS_BASE: u32 = 0x4000_8000;

#[repr(C)]
pub struct ClocksRegisters {
    pub reserved_00: [u32; 40], // 0x00..0xa0
    pub wake_en0: u32,          // 0xa0
    pub wake_en1: u32,          // 0xa4
}

const _: () = assert!(::core::mem::offset_of!(ClocksRegisters, wake_en0) == 0xa0);
const _: () = assert!(::core::mem::offset_of!(ClocksRegisters, wake_en1) == 0xa4);

macro_rules! clocks_reg {
    ($member:ident) => {
        $crate::registers::CLOCKS_BASE + ::core::mem::offset_of!($crate::registers::ClocksRegisters, $member) as u32
    };
}
pub(crate) use clocks_reg;

/// Clock enable register 0 (C `RP_CLOCKS_WAKE_EN0`).
pub const CLOCKS_WAKE_EN0: u32 = clocks_reg!(wake_en0);
/// Clock enable register 1 (C `RP_CLOCKS_WAKE_EN1`).
pub const CLOCKS_WAKE_EN1: u32 = clocks_reg!(wake_en1);
/// Clocks the boot ROM enables before it touches the flash (C `RP_CLOCKS_WAKE_EN0_MASK`).
pub const CLOCKS_WAKE_EN0_MASK: u32 = 0xff0c_0f19;
/// Same for register 1 (C `RP_CLOCKS_WAKE_EN1_MASK`).
pub const CLOCKS_WAKE_EN1_MASK: u32 = 0x0000_2007;

// --- IO_QSPI / pads -----------------------------------------------------------

pub const GPIO_QSPI_BASE: u32 = 0x4001_8000;

#[repr(C)]
pub struct GpioQspiRegisters {
    pub sclk_status: u32, // 0x00
    pub sclk_ctrl: u32,   // 0x04
    pub cs_status: u32,   // 0x08
    pub cs_ctrl: u32,     // 0x0c
    pub sd0_status: u32,  // 0x10
    pub sd0_ctrl: u32,    // 0x14
    pub sd1_status: u32,  // 0x18
    pub sd1_ctrl: u32,    // 0x1c
    pub sd2_status: u32,  // 0x20
    pub sd2_ctrl: u32,    // 0x24
    pub sd3_status: u32,  // 0x28
    pub sd3_ctrl: u32,    // 0x2c
}

const _: () = assert!(::core::mem::offset_of!(GpioQspiRegisters, sclk_ctrl) == 0x04);
const _: () = assert!(::core::mem::offset_of!(GpioQspiRegisters, cs_ctrl) == 0x0c);
const _: () = assert!(::core::mem::offset_of!(GpioQspiRegisters, sd3_ctrl) == 0x2c);

macro_rules! gpio_qspi_reg {
    ($member:ident) => {
        $crate::registers::GPIO_QSPI_BASE + ::core::mem::offset_of!($crate::registers::GpioQspiRegisters, $member) as u32
    };
}
pub(crate) use gpio_qspi_reg;

/// QSPI clock GPIO control (C `RP_GPIO_QSPI_SCLK_CTRL`).
pub const GPIO_QSPI_SCLK_CTRL: u32 = gpio_qspi_reg!(sclk_ctrl);
/// QSPI chip select GPIO control (C `RP_GPIO_QSPI_CS_CTRL`).
pub const GPIO_QSPI_CS_CTRL: u32 = gpio_qspi_reg!(cs_ctrl);
/// QSPI data line 0 GPIO control (C `RP_GPIO_QSPI_SD0_CTRL`).
pub const GPIO_QSPI_SD0_CTRL: u32 = gpio_qspi_reg!(sd0_ctrl);
/// QSPI data line 1 GPIO control (C `RP_GPIO_QSPI_SD1_CTRL`).
pub const GPIO_QSPI_SD1_CTRL: u32 = gpio_qspi_reg!(sd1_ctrl);
/// QSPI data line 2 GPIO control (C `RP_GPIO_QSPI_SD2_CTRL`).
pub const GPIO_QSPI_SD2_CTRL: u32 = gpio_qspi_reg!(sd2_ctrl);
/// QSPI data line 3 GPIO control (C `RP_GPIO_QSPI_SD3_CTRL`).
pub const GPIO_QSPI_SD3_CTRL: u32 = gpio_qspi_reg!(sd3_ctrl);
/// Chip select driven by the peripheral (reset state).
pub const GPIO_QSPI_CS_DRIVE_NORMAL: u32 = 0 << 8;
/// Chip select inverted (idle high).
pub const GPIO_QSPI_CS_DRIVE_INVERT: u32 = 1 << 8;
/// Chip select forced low.
pub const GPIO_QSPI_CS_DRIVE_LOW: u32 = 2 << 8;
/// Chip select forced high.
pub const GPIO_QSPI_CS_DRIVE_HIGH: u32 = 3 << 8;
/// Mask of the drive field (C `RP_GPIO_QSPI_CS_DRIVE_MASK`).
pub const GPIO_QSPI_CS_DRIVE_MASK: u32 = 0x0000_0300;
/// Input override of SD1, which the flash drives low while in QPI mode
/// (C `RP_GPIO_QSPI_SD1_CTRL_INOVER_BITS`).
pub const GPIO_QSPI_SD1_CTRL_INOVER_BITS: u32 = 0x0003_0000;
/// Power-on state of the QSPI clock GPIO (C `RP_GPIO_QSPI_SCLK_POR`).
pub const GPIO_QSPI_SCLK_POR: u32 = 0x0000_001f;

pub const PADS_QSPI_BASE: u32 = 0x4002_0000;

#[repr(C)]
pub struct PadsQspiRegisters {
    pub voltage_select: u32, // 0x00
    pub gpio_sclk: u32,      // 0x04
    pub gpio_sd0: u32,       // 0x08
    pub gpio_sd1: u32,       // 0x0c
    pub gpio_sd2: u32,       // 0x10
    pub gpio_sd3: u32,       // 0x14
}

const _: () = assert!(::core::mem::offset_of!(PadsQspiRegisters, gpio_sclk) == 0x04);
const _: () = assert!(::core::mem::offset_of!(PadsQspiRegisters, gpio_sd3) == 0x14);

macro_rules! pads_qspi_reg {
    ($member:ident) => {
        $crate::registers::PADS_QSPI_BASE + ::core::mem::offset_of!($crate::registers::PadsQspiRegisters, $member) as u32
    };
}
pub(crate) use pads_qspi_reg;

/// QSPI clock pad (C `RP_PADS_QSPI_GPIO_SCLK`).
pub const PADS_QSPI_GPIO_SCLK: u32 = pads_qspi_reg!(gpio_sclk);
/// QSPI data pad 0 (C `RP_PADS_QSPI_GPIO_SD0`).
pub const PADS_QSPI_GPIO_SD0: u32 = pads_qspi_reg!(gpio_sd0);
/// QSPI data pad 1 (C `RP_PADS_QSPI_GPIO_SD1`).
pub const PADS_QSPI_GPIO_SD1: u32 = pads_qspi_reg!(gpio_sd1);
/// QSPI data pad 2 (C `RP_PADS_QSPI_GPIO_SD2`).
pub const PADS_QSPI_GPIO_SD2: u32 = pads_qspi_reg!(gpio_sd2);
/// QSPI data pad 3 (C `RP_PADS_QSPI_GPIO_SD3`).
pub const PADS_QSPI_GPIO_SD3: u32 = pads_qspi_reg!(gpio_sd3);
/// Pad input enable: set when the chip is not configured (C `RP_PADS_QSPI_GPIO_SCLK_IE`).
pub const PADS_QSPI_GPIO_SCLK_IE: u32 = 0x0000_0040;
/// Pad output disable (C `RP_PADS_QSPI_GPIO_SD0_OD_BITS`).
pub const PADS_QSPI_GPIO_SD0_OD_BITS: u32 = 0x0000_0080;
/// Pad pull up (C `RP_PADS_QSPI_GPIO_SD0_PUE_BITS`).
pub const PADS_QSPI_GPIO_SD0_PUE_BITS: u32 = 0x0000_0008;
/// Pad pull down (C `RP_PADS_QSPI_GPIO_SD0_PDE_BITS`).
pub const PADS_QSPI_GPIO_SD0_PDE_BITS: u32 = 0x0000_0004;


// --- SSI (the SPI controller behind the QSPI pads) -----------------------------

pub const SSI_BASE: u32 = 0x1800_0000;

#[repr(C)]
pub struct SsiRegisters {
    pub ctrl0: u32,              // 0x00
    pub ctrl1: u32,              // 0x04
    pub enable: u32,             // 0x08
    pub reserved_0c: u32,        // 0x0c
    pub ser: u32,                // 0x10
    pub baud: u32,               // 0x14
    pub reserved_18: u32,        // 0x18
    pub reserved_1c: u32,        // 0x1c
    pub txflr: u32,              // 0x20
    pub rxflr: u32,              // 0x24
    pub sr: u32,                 // 0x28
    pub reserved_2c: [u32; 7],   // 0x2c..0x48
    pub icr: u32,                // 0x48
    pub reserved_4c: [u32; 5],   // 0x4c..0x60
    pub dr0: u32,                // 0x60
    pub reserved_64: [u32; 36],  // 0x64..0xf4
    pub xip_spi_ctrl0: u32,      // 0xf4
}

const _: () = assert!(::core::mem::offset_of!(SsiRegisters, ctrl0) == 0x00);
const _: () = assert!(::core::mem::offset_of!(SsiRegisters, enable) == 0x08);
const _: () = assert!(::core::mem::offset_of!(SsiRegisters, baud) == 0x14);
const _: () = assert!(::core::mem::offset_of!(SsiRegisters, icr) == 0x48);
const _: () = assert!(::core::mem::offset_of!(SsiRegisters, dr0) == 0x60);
const _: () = assert!(::core::mem::offset_of!(SsiRegisters, xip_spi_ctrl0) == 0xf4);

macro_rules! ssi_reg {
    ($member:ident) => {
        $crate::registers::SSI_BASE + ::core::mem::offset_of!($crate::registers::SsiRegisters, $member) as u32
    };
}
pub(crate) use ssi_reg;

/// Control register 0 (C `RP_SSI_CTRL0`).
pub const SSI_CTRL0: u32 = ssi_reg!(ctrl0);
/// Control register 1 (C `RP_SSI_CTRL1`).
pub const SSI_CTRL1: u32 = ssi_reg!(ctrl1);
/// Enable register (C `RP_SSI_ENABLE`).
pub const SSI_ENABLE: u32 = ssi_reg!(enable);
/// Slave enable register (C `RP_SSI_SER`).
pub const SSI_SER: u32 = ssi_reg!(ser);
/// Clock divider (C `RP_SSI_BAUD`).
pub const SSI_BAUD: u32 = ssi_reg!(baud);
/// Transmit FIFO level (C `RP_SSI_TXFLR`).
pub const SSI_TXFLR: u32 = ssi_reg!(txflr);
/// Receive FIFO level (C `RP_SSI_RXFLR`).
pub const SSI_RXFLR: u32 = ssi_reg!(rxflr);
/// Status register, cleared on read (C `RP_SSI_SR`).
pub const SSI_SR: u32 = ssi_reg!(sr);
/// Interrupt clear register, cleared on read (C `RP_SSI_ICR`).
pub const SSI_ICR: u32 = ssi_reg!(icr);
/// Data register (C `RP_SSI_DR0`).
pub const SSI_DR0: u32 = ssi_reg!(dr0);
/// XIP control register (C `RP_SSI_XIP_SPI_CTRL0`).
pub const SSI_XIP_SPI_CTRL0: u32 = ssi_reg!(xip_spi_ctrl0);
/// Frame format field (C `RP_SSI_CTRL0_FRF_MASK`).
pub const SSI_CTRL0_FRF_MASK: u32 = 0x0060_0000;
/// Standard 1-bit SPI frames (C `RP_SSI_CTRL0_FRF_SERIAL`).
pub const SSI_CTRL0_FRF_SERIAL: u32 = 0 << 21;
/// Transfer mode field (C `RP_SSI_CTRL0_TMOD_MASK`).
pub const SSI_CTRL0_TMOD_MASK: u32 = 0x0000_0300;
/// Transmit and receive for every frame (C `RP_SSI_CTRL0_TMOD_BIDI`).
pub const SSI_CTRL0_TMOD_BIDI: u32 = 0 << 8;
/// Receive data frames only (C `RP_SSI_CTRL0_TMOD_RX_ONLY`).
pub const SSI_CTRL0_TMOD_RX_ONLY: u32 = 2 << 8;
/// EEPROM read mode: instruction and address out, data in (C `RP_SSI_CTRL0_TMOD_EEPROM`).
pub const SSI_CTRL0_TMOD_EEPROM: u32 = 3 << 8;
/// Data frame size field (C `RP_SSI_CTRL0_DATA_BIT_MASK`).
pub const SSI_CTRL0_DATA_BIT_MASK: u32 = 0x001f_0000;
const SSI_CTRL0_DATA_BIT_SHIFT: u32 = 16;
/// Mask of the fields the driver rewrites (C `RP_SSI_CTRL0_MASK`).
pub const SSI_CTRL0_MASK: u32 = SSI_CTRL0_FRF_MASK | SSI_CTRL0_TMOD_MASK | SSI_CTRL0_DATA_BIT_MASK;

/// Data frame of `bits` clocks (C `RP_SSI_CTRL0_DATA_BITS()`).
pub const fn ssi_ctrl0_data_bits(bits: u32) -> u32 {
    (bits - 1) << SSI_CTRL0_DATA_BIT_SHIFT
}

/// Enable the SSI (C `RP_SSI_ENABLE_SSI`).
pub const SSI_ENABLE_SSI: u32 = 1 << 0;
/// XIP mode: standard SPI frames (C `RP_SSI_XIP_SPI_CTRL0_FORMAT_STD_SPI`).
pub const SSI_XIP_SPI_CTRL0_FORMAT_STD_SPI: u32 = 0 << 0;
/// XIP mode: use the frame format from CTRL0 (C `RP_SSI_XIP_SPI_CTRL0_FORMAT_FRF`).
pub const SSI_XIP_SPI_CTRL0_FORMAT_FRF: u32 = 2 << 0;
/// 8-bit instruction phase (C `RP_SSI_XIP_SPI_CTRL0_INSTR_LENGTH_8b`).
pub const SSI_XIP_SPI_CTRL0_INSTR_LENGTH_8B: u32 = 2 << 8;
/// Address phase of `bytes` bytes (C `RP_SSI_XIP_SPI_CTRL0_ADDRESS_LENGTH()`).
pub const fn ssi_xip_spi_ctrl0_address_length(bytes: u32) -> u32 {
    (bytes * 2) << 2
}
/// `cycles` wait cycles in the address phase (C `RP_SSI_XIP_SPI_CTRL0_WAIT_CYCLES()`).
pub const fn ssi_xip_spi_ctrl0_wait_cycles(cycles: u32) -> u32 {
    (cycles * 8) << 11
}
const SSI_XIP_SPI_CTRL0_XIP_CMD_SHIFT: u32 = 24;
/// The XIP read command byte (C `RP_SSI_XIP_SPI_CTRL0_XIP_CMD()`).
pub const fn ssi_xip_spi_ctrl0_xip_cmd(command: u32) -> u32 {
    command << SSI_XIP_SPI_CTRL0_XIP_CMD_SHIFT
}
/// Command and address both in serial format (C `RP_SSI_XIP_SPI_CTRL0_TRANS_1C1A`).
pub const SSI_XIP_SPI_CTRL0_TRANS_1C1A: u32 = 0 << 0;

// --- RESETS, PSM and XIP ------------------------------------------------------

pub const RESETS_BASE: u32 = 0x4000_c000;

#[repr(C)]
pub struct ResetsRegisters {
    pub reset: u32,       // 0x00
    pub reserved_04: u32, // 0x04
    pub reset_done: u32,  // 0x08
}

const _: () = assert!(::core::mem::offset_of!(ResetsRegisters, reset) == 0x00);
const _: () = assert!(::core::mem::offset_of!(ResetsRegisters, reset_done) == 0x08);

macro_rules! resets_reg {
    ($member:ident) => {
        $crate::registers::RESETS_BASE + ::core::mem::offset_of!($crate::registers::ResetsRegisters, $member) as u32
    };
}
pub(crate) use resets_reg;

/// Reset control register (C `RP_RESETS_RESET`).
pub const RESETS_RESET: u32 = resets_reg!(reset);
/// Reset done register (C `RP_RESETS_RESET_DONE`).
pub const RESETS_RESET_DONE: u32 = resets_reg!(reset_done);
/// The DMA controller (C `RP_RESETS_RESET_DMA_BITS`).
pub const RESETS_RESET_DMA_BITS: u32 = 1 << 2;
/// The QSPI IO block (C `RP_RESETS_RESET_IO_QSPI_BITS`).
pub const RESETS_RESET_IO_QSPI_BITS: u32 = 0x0000_0040;
/// The QSPI pads (C `RP_RESETS_RESET_PADS_QSPI_BITS`).
pub const RESETS_RESET_PADS_QSPI_BITS: u32 = 0x0000_0200;

pub const PSM_BASE: u32 = 0x4001_0000;

#[repr(C)]
pub struct PsmRegisters {
    pub reserved_00: u32, // 0x00
    pub frce_off: u32,    // 0x04
}

const _: () = assert!(::core::mem::offset_of!(PsmRegisters, frce_off) == 0x04);

macro_rules! psm_reg {
    ($member:ident) => {
        $crate::registers::PSM_BASE + ::core::mem::offset_of!($crate::registers::PsmRegisters, $member) as u32
    };
}
pub(crate) use psm_reg;

/// Power state machine, force-off register (C `RP_PSM_FRCE_OFF`).
pub const PSM_FRCE_OFF: u32 = psm_reg!(frce_off);
/// Core 1 (C `RP_PSM_FRCE_OFF_PROC1_BITS`).
pub const PSM_FRCE_OFF_PROC1_BITS: u32 = 1 << 16;

pub const XIP_BASE: u32 = 0x1400_0000;

#[repr(C)]
pub struct XipRegisters {
    pub ctrl: u32,  // 0x00
    pub flush: u32, // 0x04
    pub stat: u32,  // 0x08
}

const _: () = assert!(::core::mem::offset_of!(XipRegisters, ctrl) == 0x00);
const _: () = assert!(::core::mem::offset_of!(XipRegisters, stat) == 0x08);

macro_rules! xip_reg {
    ($member:ident) => {
        $crate::registers::XIP_BASE + ::core::mem::offset_of!($crate::registers::XipRegisters, $member) as u32
    };
}
pub(crate) use xip_reg;

/// XIP control register (C `RP_XIP_CTRL`).
pub const XIP_CTRL: u32 = xip_reg!(ctrl);
/// XIP cache flush trigger (C `RP_XIP_FLUSH`).
pub const XIP_FLUSH: u32 = xip_reg!(flush);
/// XIP status register (C `RP_XIP_STAT`).
pub const XIP_STAT: u32 = xip_reg!(stat);
/// Enable the XIP cache (C `RP_XIP_CTRL_ENABLE`).
pub const XIP_CTRL_ENABLE: u32 = 0x0000_0001;
/// XIP status right after reset (C `RP_XIP_STAT_POR`).
pub const XIP_STAT_POR: u32 = 0x0000_0002;

// --- Cortex-M bits the driver needs ------------------------------------------

/// Application interrupt and reset control register (C `CORTEXM_AIRCR`).
pub const CORTEXM_AIRCR: u32 = 0xe000_ed0c;
/// The `0x05fa` key of the AIRCR writes (C `CORTEXM_AIRCR_VECTKEY`).
pub const CORTEXM_AIRCR_VECTKEY: u32 = 0x05fa << 16;
/// Ask the core to reset itself (C `CORTEXM_AIRCR_SYSRESETREQ`).
pub const CORTEXM_AIRCR_SYSRESETREQ: u32 = 1 << 2;
/// Thumb bit of the program status register (C `CORTEXM_XPSR_THUMB`).
pub const CORTEXM_XPSR_THUMB: u32 = 1 << 24;

/// Register indices of the Cortex-M register file (`cortex.h`).
///
/// Link register.
pub const CORTEX_REG_LR: usize = 14;
/// Program counter.
pub const CORTEX_REG_PC: usize = 15;
/// Program status.
pub const CORTEX_REG_XPSR: usize = 16;
/// Main stack pointer.
pub const CORTEX_REG_MSP: usize = 17;
