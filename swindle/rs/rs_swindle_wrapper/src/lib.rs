#![no_std]

pub use rsbmp::*;

// Makes sure the C API of the LED crate (swindle_leds_run) ends up in the static library
extern crate rs_swindle_leds;

/// RISC-V drivers (CH32V00x / CH32V1xx-3xx). Always compiled: their driver crates are
/// unconditional dependencies of this wrapper. The module holds the probe entry point and the
/// per-family driver storage; the dispatch to the C target model is
/// `rsbmp::riscv::driver_registry`.
pub mod riscv_shim;
pub use riscv_shim::*;

/// ARM (Cortex-M) probe chain: the `switch (target->designer_code)` of `cortexm.c` moved to
/// Rust (`cortexm_probe_chain`). Always compiled: it drives the C probes of the families that
/// are not ported yet as well as the ported ones, whose driver slots live next to their probe
/// entry point (cargo features `rust_stm32f1`/`rust_stm32f4`/`rust_stm32g0`/`rust_rp2040`/
/// `rust_rp2350`, see `swindle/rs/CMakeLists.txt`). The dispatch to the C target model is
/// `rsbmp::arm::driver_registry`.
pub mod arm_probe_chain;
pub use arm_probe_chain::*;
