//! CRC32 through the DMA sniffer, so GDB's `qCRC` never has to read the range.
//!
//! Rust port of `rp_crc32()` from `blackmagic/src/target/rp2040.c`. A tiny stub parked at
//! the start of SRAM programs DMA channel 0 to stream the range through the sniffing
//! hardware and traps with the result in `r1`, which is the `target_s::crc32` hook.
//!
//! The SRAM the stub overwrites belongs to the target, so it is saved first and put back
//! afterwards (through the probe's shared scratch buffer, whose size is why the range is
//! refused when it cannot hold the stub).

use crate::registers::*;
use rs_rp_common::flashstub_crc32::{RP2040_CRC32_BIN, RP2040_CRC32_BIN_LEN};
use rsbmp::target::{CortexMAccess, temporary_buffer};

/// Shortest range worth a stub round trip: below this the plain debug-link CRC wins.
const MIN_SIZE: usize = 100;

/// Register the stub leaves the checksum in (C `target->reg_read(target, 1, ...)`).
const RESULT_REG: u32 = 1;

/// Words of SRAM the stub works in: itself, plus the word the DMA writes through
/// (C `RP_CRC_STUB_ADDR` .. `+ RP2040_CRC32_BIN_LEN + 4`).
const STUB_WINDOW: usize = RP2040_CRC32_BIN_LEN + 4;

// The CRC window must not reach the write stub's data buffer...
const _: () = assert!(CRC_STUB_ADDR + STUB_WINDOW as u32 <= STUB_BUFFER_BASE);

/// Fast CRC32 of `[addr, addr + len)` (C `rp_crc32()`).
///
/// `None` means "not this range": too short, misaligned, or overlapping the stub's own
/// scratch window. The caller then falls back to the generic debug-link CRC.
pub fn crc32(cm: &CortexMAccess, addr: u32, len: usize) -> Option<u32> {
    if len < MIN_SIZE || addr & 3 != 0 || len & 3 != 0 {
        return None;
    }
    // A range that covers the scratch window would checksum the stub's own code.
    let window_end = CRC_STUB_ADDR + STUB_WINDOW as u32;
    if addr >= CRC_STUB_ADDR && addr < window_end {
        return None;
    }
    if addr <= CRC_STUB_ADDR && addr.wrapping_add(len as u32) > CRC_STUB_ADDR {
        return None;
    }

    let saved = temporary_buffer(STUB_WINDOW)?;

    // Pulse the DMA out of reset to leave it known-state, and keep the rest as is.
    let resets = cm.read32(RESETS_RESET);
    let dma_reset = resets & !RESETS_RESET_DMA_BITS;
    cm.write32(RESETS_RESET, dma_reset | RESETS_RESET_DMA_BITS);

    // Step 1: SRAM belongs to the target, so keep what the stub overwrites.
    cm.mem_read(saved, CRC_STUB_ADDR);
    cm.write32(RESETS_RESET, dma_reset);
    // Step 2: upload the stub just past the word the DMA writes into.
    cm.mem_write(CRC_STUB_ADDR + 4, &RP2040_CRC32_BIN);

    // Step 3: run it -- it takes (length, address, scratch, scratch) and publishes the
    // checksum in `r1`, which the runner only hands out after a successful exit.
    let result = cm
        .run_stub_result(CRC_STUB_ADDR + 4, RESULT_REG, len as u32, addr, CRC_STUB_ADDR, CRC_STUB_ADDR)
        // The sniffer's output is inverted by the stub, so undo that.
        .map(|crc| crc ^ 0xffff_ffff);

    // Step 4: give the target its SRAM back.
    cm.mem_write(CRC_STUB_ADDR, saved);
    result
}
