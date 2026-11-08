//! The RP2350 driver for its Hazard3 RISC-V cores.
//!
//! The RISC-V half of `blackmagic/src/target/rp2350.c`: `rp2350_probe()` for a Hazard3 hart,
//! the memory map, the flash hooks and the DMA-sniffer CRC32, on top of the flash controller
//! in [`rs_rp2350_common`]. The Arm cores of the same chip are served by `rs_rp2350_arm`;
//! which of the two a probe reaches is decided by the designer code the RISC-V scan found
//! (see `rs_swindle_wrapper/src/riscv_shim.rs`).
//!
//! Unlike the Cortex-M side, a hart is reached through
//! [`rs_rp_common::access::HartAccess`], so the shared flash code drives both.

#![no_std]

pub mod crc32;
pub mod driver;

pub use driver::Rp2350RiscvDriver;
