//! Swindle Rust Target Subsystem.
//!
//! Provides idiomatic Rust target abstractions (`TargetDriver`, `RamRegion`),
//! along with a C-ABI translation layer (`c_adapter`) that exposes the C symbols originally
//! implemented by `blackmagic/src/target/target.c`.

pub mod driver;
pub mod jep106;
pub mod memory;
pub mod memory_map;
pub mod part_descriptor;
pub mod registry;
pub mod target;
pub mod c_adapter;
pub mod riscv_driver;
pub mod arm_driver;
pub mod cortexm_access;
pub mod breakwatch;

/// Describes a contiguous RAM region on target.
///
/// Architecture-neutral: both [`RustRiscvTargetDriver`] and
/// [`RustArmTargetDriver`] report their memory map with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RamDesc {
    pub start: u32,
    pub length: usize,
}

/// Describes a flash memory region on target.
///
/// Architecture-neutral: both [`RustRiscvTargetDriver`] and
/// [`RustArmTargetDriver`] report their memory map with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlashDesc {
    pub start: u32,
    pub length: usize,
    /// Erase block size: the granularity `erase_block()` is called with.
    pub blocksize: usize,
    /// Data the driver's write hook is asked for per call, `0` meaning [`Self::blocksize`].
    ///
    /// Deliberately allowed to be **larger** than the erase block: one call that spans several
    /// pages costs the flash manager fewer round trips, and a stub loops over the pages itself.
    /// The limit is the other way round -- it must fit the write stub's data window in target
    /// RAM (see each driver's `STUB_DATA_LOCATION` assert), because the chunk is copied there
    /// in one bulk memory write.
    pub writesize: usize,
    pub erasesizes: &'static [usize],
    pub erased: u8,
}

pub use driver::{BreakwatchType, HaltReason, TargetDriver, TargetError};
pub use breakwatch::{Breakwatch, BreakwatchTable, MAX_BREAKWATCHES};
pub use memory::{read_u16, read_u32, read_u8, write_u16, write_u32, write_u8, RamRegion};
pub use registry::{
    ActiveDriverSlot, DRIVER_STORAGE, DRIVER_STORAGE_ALIGN, DRIVER_STORAGE_SIZE, DriverSlot,
    DriverStorage, DriverTag,
};
pub use riscv_driver::RustRiscvTargetDriver;
pub use memory_map::{MemoryMap, MAX_FLASH_REGIONS, MAX_RAM_REGIONS};
pub use part_descriptor::{PartDescriptor, install_part};
pub use arm_driver::{RustArmTargetDriver, TOPT_INHIBIT_NRST, TOPT_NON_HALTING_MEM_IO};
pub use cortexm_access::{CortexMAccess, ALIGN_16BIT, ALIGN_32BIT, temporary_buffer};
pub use target::{
    Target, TargetAttach, TargetDetach, TargetMassErase, TargetView, PrivFreeHook,
    TOPT_ARCH_ARM, TOPT_ARCH_RISCV, delay_ms, get_cur_target, platform_target_clk_output_enable, set_cur_target,
    target_addr_is_flash, target_attach, target_attach_n, target_check_error,
    target_detach, target_flash_count, target_flash_get, target_list, target_list_free,
    target_new, target_ram_count, target_ram_get,
};
pub use c_adapter::{TargetC, target_mem_map_free};

/// Unified target driver dispatcher: routes pure-Rust targets (`Riscv`, `CortexM`) directly
/// through Rust.
pub enum TargetDriverRef {
    Riscv(crate::riscv::RiscvTargetDriver),
    CortexM(crate::arm::CortexMTargetDriver),
}

impl TargetDriver for TargetDriverRef {
    fn mem_read(&self, dest: &mut [u8], addr: u32) -> Result<(), TargetError> {
        match self {
            Self::Riscv(d) => d.mem_read(dest, addr),
            Self::CortexM(d) => d.mem_read(dest, addr),
        }
    }

    fn mem_write(&self, addr: u32, src: &[u8]) -> Result<(), TargetError> {
        match self {
            Self::Riscv(d) => d.mem_write(addr, src),
            Self::CortexM(d) => d.mem_write(addr, src),
        }
    }

    fn mem_read_nostop(&self, dest: &mut [u8], addr: u32) -> Result<bool, TargetError> {
        match self {
            Self::Riscv(d) => d.mem_read_nostop(dest, addr),
            Self::CortexM(d) => d.mem_read_nostop(dest, addr),
        }
    }

    fn mem_write_nostop(&self, addr: u32, src: &[u8]) -> Result<bool, TargetError> {
        match self {
            Self::Riscv(d) => d.mem_write_nostop(addr, src),
            Self::CortexM(d) => d.mem_write_nostop(addr, src),
        }
    }

    fn needs_halt_for_mem(&self) -> bool {
        match self {
            Self::Riscv(d) => d.needs_halt_for_mem(),
            Self::CortexM(d) => d.needs_halt_for_mem(),
        }
    }

    fn reg_read(&self, reg: u32, buf: &mut [u8]) -> Result<usize, TargetError> {
        match self {
            Self::Riscv(d) => d.reg_read(reg, buf),
            Self::CortexM(d) => d.reg_read(reg, buf),
        }
    }

    fn reg_write(&self, reg: u32, data: &[u8]) -> Result<usize, TargetError> {
        match self {
            Self::Riscv(d) => d.reg_write(reg, data),
            Self::CortexM(d) => d.reg_write(reg, data),
        }
    }

    fn regs_read(&self, buf: &mut [u8]) -> Result<(), TargetError> {
        match self {
            Self::Riscv(d) => d.regs_read(buf),
            Self::CortexM(d) => d.regs_read(buf),
        }
    }

    fn regs_write(&self, data: &[u8]) -> Result<(), TargetError> {
        match self {
            Self::Riscv(d) => d.regs_write(data),
            Self::CortexM(d) => d.regs_write(data),
        }
    }

    fn regs_size(&self) -> usize {
        match self {
            Self::Riscv(d) => d.regs_size(),
            Self::CortexM(d) => d.regs_size(),
        }
    }

    fn reset(&self) {
        match self {
            Self::Riscv(d) => d.reset(),
            Self::CortexM(d) => d.reset(),
        }
    }

    fn halt_request(&self) {
        match self {
            Self::Riscv(d) => d.halt_request(),
            Self::CortexM(d) => d.halt_request(),
        }
    }

    fn halt_poll(&self) -> (HaltReason, u32) {
        match self {
            Self::Riscv(d) => d.halt_poll(),
            Self::CortexM(d) => d.halt_poll(),
        }
    }

    fn halt_resume(&self, step: bool) {
        match self {
            Self::Riscv(d) => d.halt_resume(step),
            Self::CortexM(d) => d.halt_resume(step),
        }
    }

    fn attach(&mut self) -> Result<(), TargetError> {
        match self {
            Self::Riscv(d) => d.attach(),
            Self::CortexM(d) => d.attach(),
        }
    }

    fn detach(&mut self) {
        match self {
            Self::Riscv(d) => d.detach(),
            Self::CortexM(d) => d.detach(),
        }
    }

    fn breakwatch_set(&mut self, breakwatch: &mut Breakwatch) -> Result<(), TargetError> {
        match self {
            Self::Riscv(d) => d.breakwatch_set(breakwatch),
            Self::CortexM(d) => d.breakwatch_set(breakwatch),
        }
    }

    fn breakwatch_clear(&mut self, breakwatch: &Breakwatch) -> Result<(), TargetError> {
        match self {
            Self::Riscv(d) => d.breakwatch_clear(breakwatch),
            Self::CortexM(d) => d.breakwatch_clear(breakwatch),
        }
    }

    fn breakwatch_clear_all(&mut self) {
        match self {
            Self::Riscv(d) => d.breakwatch_clear_all(),
            Self::CortexM(d) => d.breakwatch_clear_all(),
        }
    }
}

/// Retrieve the active target driver.
pub fn current_driver() -> Option<TargetDriverRef> {
    driver_for(get_cur_target())
}

/// Retrieve the driver of a specific target handle (which need not be the current one).
pub fn driver_for(t: *mut Target) -> Option<TargetDriverRef> {
    if t.is_null() {
        return None;
    }
    if let Some(ctx) = crate::riscv::get_riscv_context(t) {
        Some(TargetDriverRef::Riscv(crate::riscv::RiscvTargetDriver { ctx, target: t }))
    } else if crate::arm::cortexm::is_cortexm(t) {
        Some(TargetDriverRef::CortexM(crate::arm::CortexMTargetDriver { target: t }))
    } else {
        None
    }
}
