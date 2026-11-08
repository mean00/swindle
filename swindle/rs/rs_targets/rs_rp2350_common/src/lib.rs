//! What every RP2350 driver shares: the register map and the QMI flash controller.
//!
//! The RP2350 has two kinds of core, a Cortex-M33 pair and a pair of Hazard3 RISC-V cores,
//! and a debug probe can end up driving either. Everything below them that is chip-specific
//! — the boot ROM magic, the memory map, the QMI taken into direct mode, the SPI transport
//! built out of it — is the same code for both, so it lives here and the two drivers
//! (`rs_rp2350_arm`, `rs_rp2350_riscv`) only add their own probe and the way they reach
//! target memory.
//!
//! Port of the flash half of `blackmagic/src/target/rp2350.c`. The flash controller talks to
//! the chip through [`rs_rp_common::access::FlashAccess`], which is what makes the QMI code
//! here independent of the core driving it.

#![no_std]

pub mod flash;
pub mod flashstub;
pub mod registers;
