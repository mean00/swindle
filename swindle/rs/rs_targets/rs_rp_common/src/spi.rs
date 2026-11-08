//! SPI NOR flash layer on top of a controller (C `spi.c`).
//!
//! The driver hands in an [`SpiTransport`], its own way of putting a command word on the
//! chip, and this module implements the things every SPI NOR chip has: identify it (JEDEC
//! ID + SFDP), erase a sector, erase the whole chip, program a range of pages.
//!
//! Commands are encoded like the C `SPI_FLASH_CMD_*` words: the low byte is the opcode,
//! bits 8..10 say how many dummy bytes to insert, bit 11 whether a 24-bit address follows
//! and bit 12 whether the chip is being read or written.

use crate::access::FlashAccess;
use crate::sfdp::{self, SpiParameters};

/// Mask of the opcode byte (C `SPI_FLASH_OPCODE_MASK`).
pub const OPCODE_MASK: u16 = 0x00ff;
/// Mask of the dummy byte count (C `SPI_FLASH_DUMMY_MASK`).
pub const DUMMY_MASK: u16 = 0x0700;
/// Shift of the dummy byte count (C `SPI_FLASH_DUMMY_SHIFT`).
pub const DUMMY_SHIFT: u32 = 8;
/// Mask of the address phase selector (C `SPI_FLASH_OPCODE_MODE_MASK`).
pub const OPCODE_MODE_MASK: u16 = 0x0800;
/// Address phase present (C `SPI_FLASH_OPCODE_3B_ADDR`).
pub const OPCODE_3B_ADDR: u16 = 1 << 11;
/// Data phase into the chip (C `SPI_FLASH_DATA_OUT`).
pub const DATA_OUT: u16 = 1 << 12;

/// Opcode of the 4 KiB sector erase (C `SPI_FLASH_OPCODE_SECTOR_ERASE`).
pub const SECTOR_ERASE_OPCODE: u8 = 0x20;

/// Dummy bytes phase of a command word (C `SPI_FLASH_DUMMY_LEN()`).
pub const fn dummy_len(len: u16) -> u16 {
    (len << DUMMY_SHIFT) & DUMMY_MASK
}

/// Write enable (C `SPI_FLASH_CMD_WRITE_ENABLE`).
pub const CMD_WRITE_ENABLE: u16 = 0x06;
/// Page program, 24-bit address (C `SPI_FLASH_CMD_PAGE_PROGRAM`).
pub const CMD_PAGE_PROGRAM: u16 = OPCODE_3B_ADDR | DATA_OUT | 0x02;
/// Sector erase, opcode supplied per chip (C `SPI_FLASH_CMD_SECTOR_ERASE`).
pub const CMD_SECTOR_ERASE: u16 = OPCODE_3B_ADDR;
/// Whole-chip erase (C `SPI_FLASH_CMD_CHIP_ERASE`).
pub const CMD_CHIP_ERASE: u16 = 0x60;
/// Read the status register (C `SPI_FLASH_CMD_READ_STATUS`).
pub const CMD_READ_STATUS: u16 = 0x05;
/// Read the JEDEC id (C `SPI_FLASH_CMD_READ_JEDEC_ID`).
pub const CMD_READ_JEDEC_ID: u16 = 0x9f;
/// Read the SFDP tables, one dummy byte after the address (C `SPI_FLASH_CMD_READ_SFDP`).
pub const CMD_READ_SFDP: u16 = OPCODE_3B_ADDR | dummy_len(1) | 0x5a;

/// Status register bit: an erase or program is in progress (C `SPI_FLASH_STATUS_BUSY`).
pub const STATUS_BUSY: u8 = 0x01;
/// Status register bit: writes are enabled (C `SPI_FLASH_STATUS_WRITE_ENABLED`).
pub const STATUS_WRITE_ENABLED: u8 = 0x02;

/// Erased state of the flash (all bits set).
pub const ERASED: u8 = 0xff;

/// Page size assumed when SFDP does not say (C `bmp_spi_add_flash()`).
const DEFAULT_PAGE_SIZE: usize = 256;
/// Sector size assumed when SFDP cannot be read (C `bmp_spi_add_flash()`).
const DEFAULT_SECTOR_SIZE: usize = 4096;
/// One MiB, for the size report.
#[allow(dead_code)]
const MIB: usize = 1024 * 1024;

/// A controller that can put command words on the SPI flash and get data back.
///
/// The three methods map one to one onto the C `spi_read_func`, `spi_write_func` and
/// `spi_run_command_func` callbacks a BMP driver hands to `bmp_spi_add_flash()`.
pub trait SpiTransport {
    /// Send `command` and read `buffer.len()` bytes back.
    fn read(&self, m: &dyn FlashAccess, command: u16, address: u32, buffer: &mut [u8]);
    /// Send `command` followed by `buffer`.
    fn write(&self, m: &dyn FlashAccess, command: u16, address: u32, buffer: &[u8]);
    /// Send `command` on its own, to deselect the chip and let it start.
    fn run_command(&self, m: &dyn FlashAccess, command: u16, address: u32);
}

/// The JEDEC id of a flash chip (C `spi_flash_id_s`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlashId {
    /// Manufacturer code.
    pub manufacturer: u8,
    /// Device type byte.
    pub device_type: u8,
    /// log2 of the capacity in bytes.
    pub capacity: u8,
}

impl FlashId {
    /// No chip answering: every byte reads back `0xff`.
    pub const NONE: Self = Self {
        manufacturer: 0xff,
        device_type: 0xff,
        capacity: 0xff,
    };

    /// Whether the id looks like a real chip answered (C `rp2350_add_flash()`'s test).
    pub fn is_present(&self) -> bool {
        *self != Self::NONE
    }
}

/// Read the JEDEC id of the chip behind `transport`.
pub fn read_jedec_id(m: &dyn FlashAccess, transport: &dyn SpiTransport) -> FlashId {
    let mut id = [0xffu8; 3];
    transport.read(m, CMD_READ_JEDEC_ID, 0, &mut id);
    FlashId {
        manufacturer: id[0],
        device_type: id[1],
        capacity: id[2],
    }
}

/// A flash chip mapped at `start`, with the geometry it reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiFlash {
    /// Address of the chip's first byte, as the target sees it.
    pub start: u32,
    /// Size of the chip in bytes.
    pub length: usize,
    /// Bytes per erase block.
    pub sector_size: usize,
    /// Bytes per program page.
    pub page_size: usize,
    /// Opcode of the sector erase command this chip wants.
    pub sector_erase_opcode: u8,
}


impl SpiFlash {
    /// Identify the chip behind `transport` and describe its geometry (C
    /// `bmp_spi_add_flash()`).
    ///
    /// `fallback_length` is the size to map when SFDP cannot be read: the caller's own
    /// guess, from the JEDEC id or the family maximum.
    pub fn identify(
        m: &dyn FlashAccess,
        transport: &dyn SpiTransport,
        start: u32,
        fallback_length: usize,
    ) -> Self {
        let parameters = sfdp::read_parameters(m, transport).unwrap_or_else(|| {
            // SFDP readout failed, so make some assumptions and hope for the best.
            SpiParameters {
                page_size: DEFAULT_PAGE_SIZE,
                sector_size: DEFAULT_SECTOR_SIZE,
                capacity: fallback_length,
                sector_erase_opcode: SECTOR_ERASE_OPCODE,
            }
        });

        Self {
            start,
            length: parameters.capacity,
            sector_size: parameters.sector_size,
            page_size: parameters.page_size,
            sector_erase_opcode: parameters.sector_erase_opcode,
        }
    }

    /// Read the status register of the chip (C `bmp_spi_read_status()`).
    pub fn read_status(&self, m: &dyn FlashAccess, transport: &dyn SpiTransport) -> u8 {
        let mut status = 0u8;
        transport.read(m, CMD_READ_STATUS, 0, core::slice::from_mut(&mut status));
        status
    }

    /// Enable writes, returning whether the chip took it (C `bmp_spi_flash_write()`).
    fn write_enable(&self, m: &dyn FlashAccess, transport: &dyn SpiTransport) -> bool {
        transport.run_command(m, CMD_WRITE_ENABLE, 0);
        self.read_status(m, transport) & STATUS_WRITE_ENABLED != 0
    }

    /// Wait for the erase or program in progress to finish.
    ///
    /// `false` when the debug link raised a fault on the way, which is the only way out
    /// of the C loops other than the chip clearing its busy bit.
    fn wait_ready(&self, m: &dyn FlashAccess, transport: &dyn SpiTransport) -> bool {
        while self.read_status(m, transport) & STATUS_BUSY != 0 {
            if m.check_error() {
                return false;
            }
        }
        true
    }

    /// Erase the whole chip (C `bmp_spi_mass_erase()`).
    pub fn mass_erase(&self, m: &dyn FlashAccess, transport: &dyn SpiTransport) -> bool {
        if !self.write_enable(m, transport) {
            return false;
        }
        transport.run_command(m, CMD_CHIP_ERASE, 0);
        self.wait_ready(m, transport)
    }

    /// Erase the sector holding `addr` (C `bmp_spi_flash_erase()`).
    ///
    /// The flash manager calls this once per block, so the length is not needed.
    pub fn erase(&self, m: &dyn FlashAccess, transport: &dyn SpiTransport, addr: u32) -> bool {
        if !self.write_enable(m, transport) {
            return false;
        }
        let command = CMD_SECTOR_ERASE | u16::from(self.sector_erase_opcode);
        transport.run_command(m, command, addr - self.start);
        self.wait_ready(m, transport)
    }

    /// Program `data` at `dest`, a page at a time (C `bmp_spi_flash_write()`).
    pub fn write(&self, m: &dyn FlashAccess, transport: &dyn SpiTransport, dest: u32, data: &[u8]) -> bool {
        if self.page_size == 0 {
            return false;
        }
        let begin = dest - self.start;
        let mut offset = 0;
        while offset < data.len() {
            if !self.write_enable(m, transport) {
                return false;
            }
            let amount = (data.len() - offset).min(self.page_size);
            transport.write(
                m,
                CMD_PAGE_PROGRAM,
                begin + offset as u32,
                &data[offset..offset + amount],
            );
            if !self.wait_ready(m, transport) {
                return false;
            }
            offset += self.page_size;
        }
        true
    }
}
