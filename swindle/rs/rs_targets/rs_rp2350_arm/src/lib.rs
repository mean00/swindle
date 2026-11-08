//! The RP2350 driver for its Cortex-M33 cores.
//!
//! Rust port of `blackmagic/src/target/rp2350.c`: `rp2350_probe()`, `rp2350_attach()`, the
//! memory map and the flash/CRC hooks, on top of the flash controller in
//! [`rs_rp2350_common`].
//!
//! The C file also serves the Hazard3 cores from the same probe entry point; here they have
//! their own crate (`rs_rp2350_riscv`), probed from `rs_swindle_wrapper/src/riscv_shim.rs`.

#![no_std]

pub mod crc32;
pub mod driver;

pub use driver::Rp2350Driver;
