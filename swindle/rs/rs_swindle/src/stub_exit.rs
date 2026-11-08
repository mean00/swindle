//! Exit convention shared by every flash stub, on both architectures.
//!
//! A stub always ends at a trap instruction -- Cortex-M `bkpt #imm`, RISC-V
//! `mv a0, imm` followed by `ebreak` -- and the code it publishes there means the
//! same thing everywhere:
//!
//! * [`STUB_EXIT_OK`] (`1`): the stub did its work,
//! * [`STUB_EXIT_FAIL`] (`0`): it could not.
//!
//! [`crate::arm::stub`] and [`crate::riscv::stub`] turn that into a `bool`
//! (`true` = the stub published `STUB_EXIT_OK`), and no driver re-reads the code
//! itself.
//!
//! A stub that also returns a value publishes it in a register (`r1` on Cortex-M,
//! `a2` on RISC-V); the runner hands it out only after a successful exit
//! (`arm::stub::run_stub_result()` / `riscv::stub::run_stub_with_stack_result()`).

/// Code a stub publishes on success: `bkpt #1` on Cortex-M, `a0 = 1` on RISC-V.
pub const STUB_EXIT_OK: u8 = 1;

/// Code a stub publishes when it failed: `bkpt #0` on Cortex-M, `a0 = 0` on RISC-V.
pub const STUB_EXIT_FAIL: u8 = 0;

/// Maximum time in milliseconds a stub is allowed to run before being treated as hung.
pub const STUB_TIMEOUT_MS: u32 = 10_000;

// A failure that read as a success would hide real flash errors, so the two codes
// must stay distinct.
const _: () = assert!(STUB_EXIT_OK != STUB_EXIT_FAIL);

// EOF
