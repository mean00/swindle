//! Pure Rust Target Driver Interface.
//!
//! Drivers implementing this trait have NO knowledge of C structs,
//! raw pointers, or C FFI callbacks. The central shim in `rs_swindle_wrapper`
//! translates this interface to/from BlackMagic's C target model.

use crate::riscv::RiscvHart;

/// Describes a contiguous RAM region on target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RamDesc {
    pub start: u32,
    pub length: usize,
}

/// Describes a flash memory region on target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlashDesc {
    pub start: u32,
    pub length: usize,
    pub blocksize: usize,
    pub writesize: usize,
    pub erasesizes: &'static [usize],
    pub erased: u8,
}

/// Pure Rust target driver interface.
pub trait RustTargetDriver {
    /// Human-readable driver / chip family name (e.g. "CH32V006\0" or "CH32V003\0").
    fn name(&self) -> &'static str;

    /// Chip Part ID (e.g. raw IDCODE).
    fn part_id(&self) -> u32;

    /// Target options flags (e.g. TOPT_INHIBIT_NRST).
    fn target_options(&self) -> u32 {
        0
    }

    /// Whether this core has usable hardware breakpoints.
    /// If false, software breakpoints will be handled via `sw_page_erase` / `sw_page_write`.
    fn has_hw_breakpoints(&self) -> bool {
        true
    }

    /// List of RAM regions to register with the debugger.
    fn ram_regions(&self) -> &[RamDesc];

    /// List of Flash regions to register with the debugger.
    fn flash_regions(&self) -> &[FlashDesc];

    /// Prepare flash controller (e.g. load in-SRAM flashstub).
    fn flash_prepare(&mut self, _hart: &mut RiscvHart) -> bool {
        true
    }

    /// Finalize flash controller (e.g. restore clocks/wait states).
    fn flash_done(&mut self, _hart: &mut RiscvHart) -> bool {
        true
    }

    /// Erase an aligned flash address range.
    fn flash_erase(&mut self, hart: &mut RiscvHart, addr: u32, len: usize) -> bool;

    /// Write an aligned block of data to flash.
    fn flash_write(&mut self, hart: &mut RiscvHart, dest: u32, data: &[u8]) -> bool;

    /// Page size used for software breakpoint patching.
    fn sw_page_size(&self) -> u32 {
        0
    }

    /// Erase a single page for software breakpoint insertion.
    fn sw_page_erase(&mut self, _hart: &mut RiscvHart, _addr: u32) -> bool {
        false
    }

    /// Write a single page for software breakpoint insertion.
    fn sw_page_write(&mut self, _hart: &mut RiscvHart, _addr: u32, _data: &[u8]) -> bool {
        false
    }
}
