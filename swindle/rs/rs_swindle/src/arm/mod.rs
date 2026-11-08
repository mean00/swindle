/// Cortex-M stub runner (native Rust port of the C `cortexm_run_stub()`).
pub mod stub;

/// Generic Cortex debug layer (native port of `blackmagic/src/target/cortex.c`): the
/// access-port link, the debug-window accessors and the CPUID decode.
pub mod cortex;

/// Cortex-M debug layer (native port of `blackmagic/src/target/cortexm.c`): probe, attach,
/// detach, `vector_catch`, and `CortexMTargetDriver`.
pub mod cortexm;

/// Cortex-M memory access and D-cache maintenance.
pub mod cortexm_memaccess;

/// Cortex-M banked register file transfers (`DCRSR` / `DCRDR`).
pub mod cortexm_regs;

/// Cortex-M hardware breakpoints (FPB) and watchpoints (DWT).
pub mod cortexm_breakpoint;

/// Cortex-M execution control: reset, halt request/poll/resume, fault unwinding, semihosting,
/// and the flash stub runner.
pub mod cortexm_halt;

/// Native ARM Debug Interface (ADIv5, SWD, and CoreSight) subsystem.
pub mod adi;

/// The `CORTEXM_*` register map of this layer: addresses, bit fields and register-file sizes.
pub mod cortexm_registers;

/// The C seam: safe wrappers for ADIv5 operations.
pub mod c_seam;

/// Dispatch of the ARM target driver that is currently probed: the C `target_s`
/// configuration and the flash/attach/CRC callbacks around it.
pub mod driver_registry;

pub use stub::*;
pub use driver_registry::*;
pub use cortexm::CortexMTargetDriver;
