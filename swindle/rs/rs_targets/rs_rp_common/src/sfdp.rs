//! SFDP (JESD216) reader: what the flash chip says about itself.
//!
//! Rust port of `blackmagic/src/target/sfdp.c`. The chip hands back three levels of
//! tables over the `0x5a` read command:
//!
//! - a header (`"SFDP"`, its version and how many parameter headers follow),
//! - one 8-byte header per parameter table, saying which JESD216 table it is, where it
//!   lives and how long it is,
//! - the table itself; the basic SPI parameter table (id `0xff00`) carries the density,
//!   the erase types and the page size.
//!
//! Only the fields the flash layer needs are decoded: capacity, sector size with its
//! erase opcode, and the page size.

use crate::access::FlashAccess;
use crate::spi::{CMD_READ_SFDP, SpiTransport};

/// Address of the SFDP header (C `SFDP_HEADER_ADDRESS`).
const HEADER_ADDRESS: u32 = 0;

/// Size of the SFDP header, which is also the address of the first parameter header
/// (C `SFDP_TABLE_HEADER_ADDRESS == sizeof(sfdp_header_s)`).
const HEADER_LEN: usize = 8;

/// Size of one parameter table header in the JESD216 layout.
const TABLE_HEADER_LEN: usize = 8;

/// Marker the header starts with (C `SFDP_MAGIC`).
const MAGIC: [u8; 4] = *b"SFDP";

/// Id of the basic SPI parameter table (C `SFDP_BASIC_SPI_PARAMETER_TABLE`).
const BASIC_SPI_PARAMETER_TABLE: u16 = 0xff00;

/// Enough of the basic parameter table to reach the page-size field: JESD216 requires
/// 9 DWORDs (36 bytes), JESD216A extends that to 16 DWORDs and the page size lives in
/// the 11th DWORD. A rounded-up 64 bytes covers every chip seen so far.
const BASIC_TABLE_LEN: usize = 64;

/// Byte offsets inside the basic SPI parameter table (JESD216 section 6.4).
///
/// Opcode of the sector erase the chip nominates (byte 1).
const OFFSET_SECTOR_ERASE_OPCODE: usize = 1;
/// Flash density in bits (bytes 4..8).
const OFFSET_MEMORY_DENSITY: usize = 4;
/// First of four erase type descriptors (bytes 28..36).
const OFFSET_ERASE_TYPES: usize = 28;
/// Programming timings, the high nibble being log2 of the page size (byte 40).
const OFFSET_PROGRAMMING_TIMING_AND_PAGE_SIZE: usize = 40;
/// Number of erase type descriptors in the table (C `SFDP_ERASE_TYPES`).
const ERASE_TYPES: usize = 4;
/// Bytes needed before the erase types end.
const MIN_TABLE_LEN: usize = OFFSET_ERASE_TYPES + ERASE_TYPES * 2;

/// Page size to assume when the table is too old (JESD216 < 1.5) to carry it.
const DEFAULT_PAGE_SIZE: usize = 256;

/// What the flash layer needs to program the chip (C `spi_parameters_s`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiParameters {
    /// Bytes per program page.
    pub page_size: usize,
    /// Bytes per erase block, from the erase type the chip nominates.
    pub sector_size: usize,
    /// Total size of the chip, in bytes.
    pub capacity: usize,
    /// Opcode of the sector erase command to use.
    pub sector_erase_opcode: u8,
}


/// Read the basic parameter table of the chip behind `transport`.
///
/// `None` means the chip did not answer with a usable SFDP: no magic, no basic table, or
/// no erase type matching the sector erase opcode it advertises. The caller then has to
/// fall back to its best guess, like the C `sfdp_read_parameters()` returning false.
pub fn read_parameters(m: &dyn FlashAccess, transport: &dyn SpiTransport) -> Option<SpiParameters> {
    let mut header = [0u8; HEADER_LEN];
    transport.read(m, CMD_READ_SFDP, HEADER_ADDRESS, &mut header);
    if header[..MAGIC.len()] != MAGIC {
        return None;
    }
    // The header carries the minor version first, then the major one.
    let version_minor = header[4];
    let version_major = header[5];
    let parameter_headers = usize::from(header[6]);

    // The C reader walks 0..=count, so `count` itself is a usable index.
    for index in 0..=parameter_headers {
        let address = HEADER_ADDRESS + HEADER_LEN as u32 + (TABLE_HEADER_LEN * index) as u32;
        let mut table_header = [0u8; TABLE_HEADER_LEN];
        transport.read(m, CMD_READ_SFDP, address, &mut table_header);

        let parameter_id = u16::from(table_header[7]) << 8 | u16::from(table_header[0]);
        if parameter_id != BASIC_SPI_PARAMETER_TABLE {
            continue;
        }

        let table_address =
            u32::from(table_header[4]) | u32::from(table_header[5]) << 8 | u32::from(table_header[6]) << 16;
        let table_len = usize::from(table_header[3]) * 4;
        if table_len < MIN_TABLE_LEN {
            return None;
        }
        let mut table = [0u8; BASIC_TABLE_LEN];
        let read_len = table_len.min(BASIC_TABLE_LEN);
        transport.read(m, CMD_READ_SFDP, table_address, &mut table[..read_len]);
        return basic_parameters(&table[..read_len], version_major, version_minor);
    }
    None
}

/// Decode the basic SPI parameter table (C `sfdp_read_basic_parameter_table()`).
fn basic_parameters(table: &[u8], version_major: u8, version_minor: u8) -> Option<SpiParameters> {
    let density = table
        .get(OFFSET_MEMORY_DENSITY..OFFSET_MEMORY_DENSITY + 4)?
        .try_into()
        .ok()?;
    let capacity = density_to_capacity(density)?;

    // The chip nominates one erase type as "the" sector erase; use that one. Failing
    // when none of them carries the opcode it advertises keeps the caller from having to
    // work with a zero-sized sector (the C reader would hand one out).
    let sector_erase_opcode = table[OFFSET_SECTOR_ERASE_OPCODE];
    let mut sector_size = 0;
    for index in 0..ERASE_TYPES {
        let erase_type = OFFSET_ERASE_TYPES + index * 2;
        let (size_exponent, opcode) = (table[erase_type], table[erase_type + 1]);
        if opcode == sector_erase_opcode {
            sector_size = 1usize << size_exponent;
            break;
        }
    }
    if sector_size == 0 {
        return None;
    }

    // The page size only exists from JESD216A on, marked as version 1.5.
    let page_size = if version_major > 1 || (version_major == 1 && version_minor >= 5) {
        match table.get(OFFSET_PROGRAMMING_TIMING_AND_PAGE_SIZE) {
            Some(&timing) => 1usize << (timing >> 4),
            None => DEFAULT_PAGE_SIZE,
        }
    } else {
        DEFAULT_PAGE_SIZE
    };

    Some(SpiParameters {
        page_size,
        sector_size,
        capacity,
        sector_erase_opcode,
    })
}

/// Capacity in bytes of a JESD216 density field (C `sfdp_memory_density_to_capacity_bits()`).
///
/// The top bit of the field selects between exponential (`2^n` bits) and linear
/// (`n + 1` bits) densities, both counted in bits.
fn density_to_capacity(density: [u8; 4]) -> Option<usize> {
    let value = u32::from(density[0])
        | u32::from(density[1]) << 8
        | u32::from(density[2]) << 16
        | u32::from(density[3] & 0x7f) << 24;
    let bits = if density[3] & 0x80 != 0 {
        1u64.checked_shl(value)?
    } else {
        u64::from(value) + 1
    };
    // The probe has a 32-bit address space, so anything larger cannot be mapped anyway.
    usize::try_from(bits >> 3).ok()
}
