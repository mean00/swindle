//! Register map of the RP2350 that the flash driver touches.
//!
//! Port of the `#define`s at the top of `blackmagic/src/target/rp2350.c`.

/// Part number the probe reports for the Arm cores (C `ID_RP2350_ARM`).
pub const PART_ID_ARM: u32 = 0x0040;
/// Part number the probe reports for the Hazard3 cores (C `ID_RP2350_RISCV`).
pub const PART_ID_RISCV: u32 = 0x0004;

// --- Boot ROM -----------------------------------------------------------------

/// Where the boot ROM stamps its magic and version (C `RP2350_BOOTROM_MAGIC`).
pub const BOOTROM_MAGIC: u32 = 0x0000_0010;
/// The `"Mu"` magic, version 2, in bits 16..24 (C `RP2350_BOOTROM_MAGIC_VALUE`).
pub const BOOTROM_MAGIC_VALUE: u32 = b'M' as u32 | ((b'u' as u32) << 8) | (2 << 16);
/// Everything but the version byte (C `RP2350_BOOTROM_MAGIC_MASK`).
pub const BOOTROM_MAGIC_MASK: u32 = 0x00ff_ffff;
/// Shift of the version byte (C `RP2350_BOOTROM_VERSION_SHIFT`).
pub const BOOTROM_VERSION_SHIFT: u32 = 24;

// --- Memory map ---------------------------------------------------------------

/// Where the QSPI flash is mapped (C `RP2350_XIP_FLASH_BASE`).
pub const XIP_FLASH_BASE: u32 = 0x1000_0000;
/// The XIP cache control window (C `RP2350_XIP_CACHE_BASE`).
pub const XIP_CACHE_BASE: u32 = 0x1800_0000;
/// Largest flash window the XIP block decodes (C `RP2350_XIP_FLASH_SIZE`).
pub const XIP_FLASH_SIZE: usize = 0x0400_0000;
/// Start of the 520 KiB SRAM (C `RP2350_SRAM_BASE`).
pub const SRAM_BASE: u32 = 0x2000_0000;
/// Size of the SRAM (C `RP2350_SRAM_SIZE`).
pub const SRAM_SIZE: usize = 0x0008_2000;

/// Address the CRC stub is loaded at (C `RP2350_CRC_STUB_ADDR`).
pub const CRC_STUB_ADDR: u32 = SRAM_BASE;

/// Flashstub workspace, laid out so the windows cannot overlap:
///
/// * `SRAM_BASE` .. `STUB_STACK_LOCATION`: the CRC stub and its saved window (positioned by
///   [`CRC_STUB_ADDR`]; the CRC drivers assert their window ends before the stack region),
/// * `STUB_STACK_LOCATION` .. `STUB_STACKEND_LOCATION`: the stack the flash writer gives the
///   write stub,
/// * `STUB_CODE_LOCATION_WRITE` .. `STUB_DATA_LOCATION`: the write stub image,
/// * `STUB_DATA_LOCATION` .. `+ ATOMIC_FLASHSTUB_WRITE`: the chunk it programs.
pub const STUB_STACK_LOCATION: u32 = SRAM_BASE + 0x0800; // 0x2000_0800
pub const STUB_STACKEND_LOCATION: u32 = SRAM_BASE + 0x0ff0; // 0x2000_0ff0
pub const STUB_CODE_LOCATION_WRITE: u32 = SRAM_BASE + 0x1000; // 0x2000_1000
pub const STUB_DATA_LOCATION: u32 = SRAM_BASE + 0x1800; // 0x2000_1800
pub const ATOMIC_FLASHSTUB_WRITE: usize = 4096;

// The workspace windows must stay disjoint: a stub image that outgrows its slot would
// otherwise be silently overwritten by its neighbour.
const _: () = assert!(CRC_STUB_ADDR + crate::flashstub::RP2350_CRC32_ARM_BIN_LEN as u32 <= STUB_STACK_LOCATION);
const _: () = assert!(CRC_STUB_ADDR + crate::flashstub::RP2350_CRC32_RISCV_BIN_LEN as u32 <= STUB_STACK_LOCATION);
const _: () = assert!(STUB_STACK_LOCATION < STUB_STACKEND_LOCATION);
const _: () = assert!(STUB_STACKEND_LOCATION <= STUB_CODE_LOCATION_WRITE);
const _: () = assert!(STUB_CODE_LOCATION_WRITE + crate::flashstub::RP2350_WRITE_ARM_BIN_LEN as u32 <= STUB_DATA_LOCATION);
const _: () = assert!(
    STUB_CODE_LOCATION_WRITE + crate::flashstub::RP2350_WRITE_RISCV_BIN_LEN as u32 <= STUB_DATA_LOCATION
);
const _: () = assert!(STUB_DATA_LOCATION + ATOMIC_FLASHSTUB_WRITE as u32 <= SRAM_BASE + SRAM_SIZE as u32);

// --- Register access modes (C `RP2350_REG_ACCESS_*`) ---------------------------

/// Plain read/write.
pub const REG_ACCESS_NORMAL: u32 = 0x0000;
/// Atomic bit set on write.
pub const REG_ACCESS_WRITE_ATOMIC_BITSET: u32 = 0x2000;
/// Atomic bit clear on write.
pub const REG_ACCESS_WRITE_ATOMIC_BITCLR: u32 = 0x3000;

// --- XIP cache maintenance (C `RP2350_XIP_CACHE_*`) ----------------------------

/// Invalidate one cache way.
pub const XIP_CACHE_INVALIDATE_BY_SET_WAY: u32 = 0;
/// Clean one cache way.
pub const XIP_CACHE_CLEAN_BY_SET_WAY: u32 = 1;
/// Invalidate the line holding an address.
pub const XIP_CACHE_INVALIDATE_BY_ADDRESS: u32 = 2;
/// Clean the line holding an address.
pub const XIP_CACHE_CLEAN_BY_ADDRESS: u32 = 3;


// --- RESETS -------------------------------------------------------------------

pub const RESETS_BASE: u32 = 0x4002_0000;

#[repr(C)]
pub struct ResetsRegisters {
    pub reset: u32,        // 0x000
    pub reserved_004: u32, // 0x004
    pub reset_done: u32,   // 0x008
}

const _: () = assert!(::core::mem::offset_of!(ResetsRegisters, reset) == 0x00);
const _: () = assert!(::core::mem::offset_of!(ResetsRegisters, reset_done) == 0x08);

#[macro_export]
macro_rules! resets_reg {
    ($member:ident) => {
        $crate::registers::RESETS_BASE + ::core::mem::offset_of!($crate::registers::ResetsRegisters, $member) as u32
    };
}
pub use resets_reg;

/// Reset control register (C `RP2350_RESETS_RESET`).
pub const RESETS_RESET: u32 = resets_reg!(reset);
/// Reset done register (C `RP2350_RESETS_RESET_DONE`).
pub const RESETS_RESET_DONE: u32 = resets_reg!(reset_done);
/// The DMA controller (C `RP2350_RESETS_RESET_DMA_BITS`).
pub const RESETS_RESET_DMA_BITS: u32 = 1 << 2;
/// The QSPI IO block (C `RP2350_RESETS_RESET_IO_QSPI`).
pub const RESETS_RESET_IO_QSPI: u32 = 1 << 7;
/// The QSPI pads (C `RP2350_RESETS_RESET_PADS_QSPI`).
pub const RESETS_RESET_PADS_QSPI: u32 = 1 << 10;

// --- IO_QSPI / pads -----------------------------------------------------------

pub const GPIO_QSPI_BASE: u32 = 0x4003_0000;

#[repr(C)]
pub struct GpioQspiRegisters {
    pub reserved_00: [u32; 5], // 0x000..0x014
    pub sclk_ctrl: u32,         // 0x014
    pub cs_status: u32,         // 0x018
    pub cs_ctrl: u32,           // 0x01c
    pub sd0_status: u32,        // 0x020
    pub sd0_ctrl: u32,          // 0x024
    pub sd1_status: u32,        // 0x028
    pub sd1_ctrl: u32,          // 0x02c
    pub sd2_status: u32,        // 0x030
    pub sd2_ctrl: u32,          // 0x034
    pub sd3_status: u32,        // 0x038
    pub sd3_ctrl: u32,          // 0x03c
}

const _: () = assert!(::core::mem::offset_of!(GpioQspiRegisters, sclk_ctrl) == 0x014);
const _: () = assert!(::core::mem::offset_of!(GpioQspiRegisters, cs_ctrl) == 0x01c);
const _: () = assert!(::core::mem::offset_of!(GpioQspiRegisters, sd3_ctrl) == 0x03c);

#[macro_export]
macro_rules! gpio_qspi_reg {
    ($member:ident) => {
        $crate::registers::GPIO_QSPI_BASE + ::core::mem::offset_of!($crate::registers::GpioQspiRegisters, $member) as u32
    };
}
pub use gpio_qspi_reg;

/// QSPI clock GPIO control (C `RP2350_GPIO_QSPI_SCLK_CTRL`).
pub const GPIO_QSPI_SCLK_CTRL: u32 = gpio_qspi_reg!(sclk_ctrl);
/// QSPI chip select GPIO control (C `RP2350_GPIO_QSPI_CS_CTRL`).
pub const GPIO_QSPI_CS_CTRL: u32 = gpio_qspi_reg!(cs_ctrl);
/// QSPI data line 0 GPIO control (C `RP2350_GPIO_QSPI_SD0_CTRL`).
pub const GPIO_QSPI_SD0_CTRL: u32 = gpio_qspi_reg!(sd0_ctrl);
/// QSPI data line 1 GPIO control (C `RP2350_GPIO_QSPI_SD1_CTRL`).
pub const GPIO_QSPI_SD1_CTRL: u32 = gpio_qspi_reg!(sd1_ctrl);
/// QSPI data line 2 GPIO control (C `RP2350_GPIO_QSPI_SD2_CTRL`).
pub const GPIO_QSPI_SD2_CTRL: u32 = gpio_qspi_reg!(sd2_ctrl);
/// QSPI data line 3 GPIO control (C `RP2350_GPIO_QSPI_SD3_CTRL`).
pub const GPIO_QSPI_SD3_CTRL: u32 = gpio_qspi_reg!(sd3_ctrl);
/// Mask of the function select field (C `RP2350_GPIO_QSPI_CTRL_FUNCSEL_MASK`).
pub const GPIO_QSPI_CTRL_FUNCSEL_MASK: u32 = 0x1f;
/// Function select value that leaves the pad unused (C `..._FUNCSEL_NONE`).
pub const GPIO_QSPI_CTRL_FUNCSEL_NONE: u32 = 0x1f;
/// Mask of the input override field (C `RP2350_GPIO_QSPI_CTRL_INOVER_MASK`).
pub const GPIO_QSPI_CTRL_INOVER_MASK: u32 = 0x3 << 16;
/// Input neither inverted nor forced (C `RP2350_GPIO_QSPI_CTRL_INOVER_NONE`).
pub const GPIO_QSPI_CTRL_INOVER_NONE: u32 = 0x0 << 16;

pub const PADS_QSPI_BASE: u32 = 0x4004_0000;

#[repr(C)]
pub struct PadsQspiRegisters {
    pub voltage_select: u32, // 0x000
    pub sclk: u32,           // 0x004
    pub sd0: u32,            // 0x008
    pub sd1: u32,            // 0x00c
    pub sd2: u32,            // 0x010
    pub sd3: u32,            // 0x014
    pub cs: u32,             // 0x018
}

const _: () = assert!(::core::mem::offset_of!(PadsQspiRegisters, sclk) == 0x004);
const _: () = assert!(::core::mem::offset_of!(PadsQspiRegisters, cs) == 0x018);

#[macro_export]
macro_rules! pads_qspi_reg {
    ($member:ident) => {
        $crate::registers::PADS_QSPI_BASE + ::core::mem::offset_of!($crate::registers::PadsQspiRegisters, $member) as u32
    };
}
pub use pads_qspi_reg;

/// QSPI clock pad (C `RP2350_PADS_QSPI_SCLK`).
pub const PADS_QSPI_SCLK: u32 = pads_qspi_reg!(sclk);
/// QSPI data pad 0 (C `RP2350_PADS_QSPI_SD0`).
pub const PADS_QSPI_SD0: u32 = pads_qspi_reg!(sd0);
/// QSPI data pad 1 (C `RP2350_PADS_QSPI_SD1`).
pub const PADS_QSPI_SD1: u32 = pads_qspi_reg!(sd1);
/// QSPI data pad 2 (C `RP2350_PADS_QSPI_SD2`).
pub const PADS_QSPI_SD2: u32 = pads_qspi_reg!(sd2);
/// QSPI data pad 3 (C `RP2350_PADS_QSPI_SD3`).
pub const PADS_QSPI_SD3: u32 = pads_qspi_reg!(sd3);
/// QSPI chip select pad (C `RP2350_PADS_QSPI_CS`).
pub const PADS_QSPI_CS: u32 = pads_qspi_reg!(cs);
/// Pad pull down (C `RP2350_PADS_QSPI_GPIO_PULL_DOWN_ENABLE`).
pub const PADS_QSPI_GPIO_PULL_DOWN_ENABLE: u32 = 1 << 2;
/// Pad pull up (C `RP2350_PADS_QSPI_GPIO_PULL_UP_ENABLE`).
pub const PADS_QSPI_GPIO_PULL_UP_ENABLE: u32 = 1 << 3;
/// Pad input enable (C `RP2350_PADS_QSPI_GPIO_INPUT_ENABLE`).
pub const PADS_QSPI_GPIO_INPUT_ENABLE: u32 = 1 << 6;
/// Pad output disable (C `RP2350_PADS_QSPI_GPIO_OUTPUT_DISABLE`).
pub const PADS_QSPI_GPIO_OUTPUT_DISABLE: u32 = 1 << 7;
/// Pad isolation: while set the QMI cannot see the pin
/// (C `RP2350_PADS_QSPI_GPIO_ISOLATE`).
pub const PADS_QSPI_GPIO_ISOLATE: u32 = 1 << 8;


// --- QMI (the SPI controller behind the QSPI pads) -----------------------------

pub const QMI_BASE: u32 = 0x400d_0000;

#[repr(C)]
pub struct QmiRegisters {
    pub direct_csr: u32, // 0x000
    pub direct_tx: u32,  // 0x004
    pub direct_rx: u32,  // 0x008
}

const _: () = assert!(::core::mem::offset_of!(QmiRegisters, direct_csr) == 0x00);
const _: () = assert!(::core::mem::offset_of!(QmiRegisters, direct_rx) == 0x08);

#[macro_export]
macro_rules! qmi_reg {
    ($member:ident) => {
        $crate::registers::QMI_BASE + ::core::mem::offset_of!($crate::registers::QmiRegisters, $member) as u32
    };
}
pub use qmi_reg;

/// Direct mode control and status (C `RP2350_QMI_DIRECT_CSR`).
pub const QMI_DIRECT_CSR: u32 = qmi_reg!(direct_csr);
/// Direct mode transmit register (C `RP2350_QMI_DIRECT_TX`).
pub const QMI_DIRECT_TX: u32 = qmi_reg!(direct_tx);
/// Direct mode receive register (C `RP2350_QMI_DIRECT_RX`).
pub const QMI_DIRECT_RX: u32 = qmi_reg!(direct_rx);
/// Direct access mode enabled (C `RP2350_QMI_DIRECT_CSR_DIRECT_ENABLE`).
pub const QMI_DIRECT_CSR_DIRECT_ENABLE: u32 = 1 << 0;
/// A transaction is still in progress (C `RP2350_QMI_DIRECT_CSR_BUSY`).
pub const QMI_DIRECT_CSR_BUSY: u32 = 1 << 1;
/// Chip select 0 asserted (C `RP2350_QMI_DIRECT_CSR_ASSERT_CS0N`).
pub const QMI_DIRECT_CSR_ASSERT_CS0N: u32 = 1 << 2;
/// Chip select 1 asserted (C `RP2350_QMI_DIRECT_CSR_ASSERT_CS1N`).
pub const QMI_DIRECT_CSR_ASSERT_CS1N: u32 = 1 << 3;
/// Chip select 0 driven by the peripheral (C `RP2350_QMI_DIRECT_CSR_AUTO_CS0N`).
pub const QMI_DIRECT_CSR_AUTO_CS0N: u32 = 1 << 6;
/// Chip select 1 driven by the peripheral (C `RP2350_QMI_DIRECT_CSR_AUTO_CS1N`).
pub const QMI_DIRECT_CSR_AUTO_CS1N: u32 = 1 << 7;
/// Transmit FIFO full (C `RP2350_QMI_DIRECT_CSR_TXFULL`).
pub const QMI_DIRECT_CSR_TXFULL: u32 = 1 << 10;
/// Transmit FIFO empty (C `RP2350_QMI_DIRECT_CSR_TXEMPTY`).
pub const QMI_DIRECT_CSR_TXEMPTY: u32 = 1 << 11;
/// Receive FIFO empty (C `RP2350_QMI_DIRECT_CSR_RXEMPTY`).
pub const QMI_DIRECT_CSR_RXEMPTY: u32 = 1 << 16;
/// Receive FIFO full (C `RP2350_QMI_DIRECT_CSR_RXFULL`).
pub const QMI_DIRECT_CSR_RXFULL: u32 = 1 << 17;
/// Clock divider used for direct mode (C `RP2350_QMI_DIRECT_CSR_CLKDIV_DEFAULT`).
pub const QMI_DIRECT_CSR_CLKDIV_DEFAULT: u32 = 12 << 22;
/// Single-bit frames (C `RP2350_QMI_DIRECT_TX_MODE_SINGLE`).
pub const QMI_DIRECT_TX_MODE_SINGLE: u32 = 0x0 << 16;
/// Quad-bit frames (C `RP2350_QMI_DIRECT_TX_MODE_QUAD`).
pub const QMI_DIRECT_TX_MODE_QUAD: u32 = 0x3 << 16;
/// 8-bit data frames (C `RP2350_QMI_DIRECT_TX_DATA_8BIT`).
pub const QMI_DIRECT_TX_DATA_8BIT: u32 = 0 << 18;
/// 16-bit data frames (C `RP2350_QMI_DIRECT_TX_DATA_16BIT`).
pub const QMI_DIRECT_TX_DATA_16BIT: u32 = 1 << 18;
/// Drive the lines in use as outputs (C `..._TX_OUTPUT_ENABLE`).
pub const QMI_DIRECT_TX_OUTPUT_ENABLE: u32 = 1 << 19;
/// Do not push received data into the RX FIFO (C `RP2350_QMI_DIRECT_TX_NOPUSH_RX`).
pub const QMI_DIRECT_TX_NOPUSH_RX: u32 = 1 << 20;
