//! The memory map a probed part reports, shared by both driver traits.
//!
//! It is built **at attach time** by the driver, from what the probe found, and returned by
//! value: the regions only have to exist while the shim turns them into C
//! `target_ram_s`/`target_flash_s` nodes. That keeps geometry out of the driver's own storage
//! (the shared 256-byte `DRIVER_STORAGE` cell, see `registry.rs`) — the driver describes the
//! part, it does not carry a copy of the map.

use super::{FlashDesc, RamDesc};

/// Largest number of flash regions a single part can report: the F76x in dual-bank
/// mode lists bank 1, its ITCM alias, bank 2 and bank 2's ITCM alias, three sector
/// groups each.
pub const MAX_FLASH_REGIONS: usize = 12;

/// Largest number of RAM regions a single part can report: ITCM, DTCM and AHB SRAM on
/// the F7; CCM plus AHB SRAM on the F4 parts.
pub const MAX_RAM_REGIONS: usize = 3;

/// No extra (non-block-sized) erase sizes: the C `erasesizes` list only needs its 0
/// terminator on the STM32F1-style parts.
static NO_EXTRA_ERASE_SIZES: [usize; 1] = [0];

/// Memory map of a probed part, returned by value by
/// [`RustArmTargetDriver::memory_map`](super::arm_driver::RustArmTargetDriver::memory_map)
/// and [`RustRiscvTargetDriver::memory_map`](super::riscv_driver::RustRiscvTargetDriver::memory_map).
#[derive(Debug, Clone, Copy)]
pub struct MemoryMap {
    pub flash: [FlashDesc; MAX_FLASH_REGIONS],
    pub flash_len: usize,
    pub ram: [RamDesc; MAX_RAM_REGIONS],
    pub ram_len: usize,
}

impl MemoryMap {
    /// Empty map, for drivers that build their regions incrementally.
    pub const fn new() -> Self {
        Self {
            flash: [FlashDesc {
                start: 0,
                length: 0,
                blocksize: 0,
                writesize: 0,
                erasesizes: &NO_EXTRA_ERASE_SIZES,
                erased: 0xff,
            }; MAX_FLASH_REGIONS],
            flash_len: 0,
            ram: [RamDesc { start: 0, length: 0 }; MAX_RAM_REGIONS],
            ram_len: 0,
        }
    }

    /// Append a flash region.
    pub fn with_flash(mut self, start: u32, length: usize, blocksize: usize, writesize: usize) -> Self {
        if self.flash_len < self.flash.len() {
            self.flash[self.flash_len] = FlashDesc {
                start,
                length,
                blocksize,
                writesize,
                erasesizes: &NO_EXTRA_ERASE_SIZES,
                erased: 0xff,
            };
            self.flash_len += 1;
        }
        self
    }

    /// Append a flash region with an explicit erase-size list.
    pub fn with_flash_desc(mut self, desc: FlashDesc) -> Self {
        if self.flash_len < self.flash.len() {
            self.flash[self.flash_len] = desc;
            self.flash_len += 1;
        }
        self
    }

    /// Append a RAM region.
    pub fn with_ram(mut self, start: u32, length: usize) -> Self {
        if self.ram_len < self.ram.len() {
            self.ram[self.ram_len] = RamDesc { start, length };
            self.ram_len += 1;
        }
        self
    }

    /// The registered flash regions.
    pub fn flash_regions(&self) -> &[FlashDesc] {
        &self.flash[..self.flash_len]
    }

    /// The registered RAM regions.
    pub fn ram_regions(&self) -> &[RamDesc] {
        &self.ram[..self.ram_len]
    }
}

impl Default for MemoryMap {
    fn default() -> Self {
        Self::new()
    }
}
