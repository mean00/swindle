//! Swindle Rust Target Subsystem.
//!
//! Provides idiomatic Rust target abstractions (`TargetDriver`, `TargetRegistry`, `RamRegion`),
//! along with a C-ABI translation layer (`c_adapter`) that exposes the 35+ C symbols originally
//! implemented by `blackmagic/src/target/target.c`.

pub mod driver;
pub mod memory;
pub mod registry;
pub mod c_adapter;
pub mod rust_driver;

pub use driver::{BreakwatchType, HaltReason, TargetDriver, TargetError};
pub use memory::{read_u16, read_u32, read_u8, write_u16, write_u32, write_u8, RamRegion};
pub use registry::TargetRegistry;
pub use rust_driver::{FlashDesc, RamDesc, RustTargetDriver};
pub use c_adapter::{
    CTargetDriver, SwBreakpointHelpersC, TargetC, target_add_default_commands_c,
    target_add_flash_region, target_add_ram32, target_flash_alloc_c, target_list,
    target_mem_map_free, target_set_driver_c, target_set_no_hw_breakpoint_c,
    target_set_options_c, target_set_part_id_c, target_set_sw_breakpoint_helpers_c,
};

/// Unified target driver dispatcher: prioritizes first-class native Rust drivers
/// (e.g. RISC-V), and falls back to C shims (`CTargetDriver`) for legacy C targets.
pub enum TargetDriverRef {
    Riscv(crate::riscv::RiscvTargetDriver),
    C(CTargetDriver),
}

impl TargetDriver for TargetDriverRef {
    fn mem_read(&self, dest: &mut [u8], addr: u32) -> Result<(), TargetError> {
        match self {
            Self::Riscv(d) => d.mem_read(dest, addr),
            Self::C(d) => d.mem_read(dest, addr),
        }
    }

    fn mem_write(&self, addr: u32, src: &[u8]) -> Result<(), TargetError> {
        match self {
            Self::Riscv(d) => d.mem_write(addr, src),
            Self::C(d) => d.mem_write(addr, src),
        }
    }

    fn mem_read_nostop(&self, dest: &mut [u8], addr: u32) -> Result<bool, TargetError> {
        match self {
            Self::Riscv(d) => d.mem_read_nostop(dest, addr),
            Self::C(d) => d.mem_read_nostop(dest, addr),
        }
    }

    fn mem_write_nostop(&self, addr: u32, src: &[u8]) -> Result<bool, TargetError> {
        match self {
            Self::Riscv(d) => d.mem_write_nostop(addr, src),
            Self::C(d) => d.mem_write_nostop(addr, src),
        }
    }

    fn needs_halt_for_mem(&self) -> bool {
        match self {
            Self::Riscv(d) => d.needs_halt_for_mem(),
            Self::C(d) => d.needs_halt_for_mem(),
        }
    }

    fn reg_read(&self, reg: u32, buf: &mut [u8]) -> Result<usize, TargetError> {
        match self {
            Self::Riscv(d) => d.reg_read(reg, buf),
            Self::C(d) => d.reg_read(reg, buf),
        }
    }

    fn reg_write(&self, reg: u32, data: &[u8]) -> Result<usize, TargetError> {
        match self {
            Self::Riscv(d) => d.reg_write(reg, data),
            Self::C(d) => d.reg_write(reg, data),
        }
    }

    fn regs_read(&self, buf: &mut [u8]) -> Result<(), TargetError> {
        match self {
            Self::Riscv(d) => d.regs_read(buf),
            Self::C(d) => d.regs_read(buf),
        }
    }

    fn regs_write(&self, data: &[u8]) -> Result<(), TargetError> {
        match self {
            Self::Riscv(d) => d.regs_write(data),
            Self::C(d) => d.regs_write(data),
        }
    }

    fn regs_size(&self) -> usize {
        match self {
            Self::Riscv(d) => d.regs_size(),
            Self::C(d) => d.regs_size(),
        }
    }

    fn reset(&self) {
        match self {
            Self::Riscv(d) => d.reset(),
            Self::C(d) => d.reset(),
        }
    }

    fn halt_request(&self) {
        match self {
            Self::Riscv(d) => d.halt_request(),
            Self::C(d) => d.halt_request(),
        }
    }

    fn halt_poll(&self) -> (HaltReason, u32) {
        match self {
            Self::Riscv(d) => d.halt_poll(),
            Self::C(d) => d.halt_poll(),
        }
    }

    fn halt_resume(&self, step: bool) {
        match self {
            Self::Riscv(d) => d.halt_resume(step),
            Self::C(d) => d.halt_resume(step),
        }
    }

    fn attach(&mut self) -> Result<(), TargetError> {
        match self {
            Self::Riscv(d) => d.attach(),
            Self::C(d) => d.attach(),
        }
    }

    fn detach(&mut self) {
        match self {
            Self::Riscv(d) => d.detach(),
            Self::C(d) => d.detach(),
        }
    }
}

/// Retrieve the active target driver.
///
/// Returns a native Rust driver if available (e.g. RISC-V), or falls back to
/// `CTargetDriver` when using a legacy C target.
pub fn current_driver() -> Option<TargetDriverRef> {
    let t = c_adapter::get_cur_target();
    if t.is_null() {
        return None;
    }
    if let Some(ctx) = crate::riscv::get_riscv_context(t) {
        Some(TargetDriverRef::Riscv(crate::riscv::RiscvTargetDriver { ctx, target: t }))
    } else {
        Some(TargetDriverRef::C(unsafe { CTargetDriver::new(t) }))
    }
}

