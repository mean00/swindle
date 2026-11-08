//! CRC32 through the DMA sniffer, so GDB's `qCRC` never has to read the range.
//!
//! ARM half of the RP2350 DMA sniffer CRC32; the Hazard3 half lives in `rs_rp2350_riscv::crc32`.
//!
//! The stub programs DMA channel 0 to stream the range through the sniffing hardware and
//! traps with the result in `r1`. The SRAM it overwrites belongs to the target, so it is
//! saved and put back through the probe's shared scratch buffer.

use rs_rp2350_common::flashstub::{RP2350_CRC32_ARM_BIN, RP2350_CRC32_ARM_BIN_LEN};
use rs_rp2350_common::registers::*;
use rsbmp::target::{CortexMAccess, temporary_buffer};

/// Below this the software CRC is cheaper than a stub round trip (C `RP2350_CRC_MIN_SIZE`).
const MIN_SIZE: usize = 300;

/// Register the stub leaves the checksum in.
const RESULT_REG: u32 = 1;

/// Stack allocated within the saved SRAM window for the stub.
const STUB_STACK_SIZE: usize = 64;

/// Words of SRAM the stub works in: itself, plus the word the DMA writes through,
/// plus a small stack window.
const STUB_WINDOW: usize = (RP2350_CRC32_ARM_BIN_LEN + 4 + STUB_STACK_SIZE + 3) & !3;

// The stub, the DMA's scratch word and the stack it runs on have to stay below the
// flashstub stack region, or a CRC run would walk over the flash writer's workspace.
const _: () = assert!(CRC_STUB_ADDR + STUB_WINDOW as u32 <= STUB_STACK_LOCATION);

/// Fast CRC32 of `[addr, addr + len)` (C `rp2350_crc32()`).
///
/// `None` means "not this range": too short, misaligned, or overlapping the stub's own
/// scratch window. The caller then falls back to the generic debug-link CRC.
pub fn crc32(cm: &CortexMAccess, addr: u32, len: usize) -> Option<u32> {
    if len < MIN_SIZE || addr & 3 != 0 || len & 3 != 0 {
        return None;
    }
    // A range that covers the scratch window would checksum the stub's own code.
    if addr < CRC_STUB_ADDR + STUB_WINDOW as u32 && addr.wrapping_add(len as u32) > CRC_STUB_ADDR {
        return None;
    }

    let saved = temporary_buffer(STUB_WINDOW)?;

    // Pulse the DMA out of reset for the transfer, restoring the state afterwards.
    let resets = cm.read32(resets_reg!(reset));
    let dma_released = resets & !RESETS_RESET_DMA_BITS;
    cm.write32(resets_reg!(reset), dma_released | RESETS_RESET_DMA_BITS);

    // Step 1: SRAM belongs to the target, so keep what the stub overwrites.
    cm.mem_read(saved, CRC_STUB_ADDR);
    cm.write32(resets_reg!(reset), dma_released);
    while cm.read32(resets_reg!(reset_done)) & RESETS_RESET_DMA_BITS == 0 {}
    // Step 2: upload the stub just past the word the DMA writes into.
    cm.mem_write(CRC_STUB_ADDR + 4, RP2350_CRC32_ARM_BIN);

    // Step 3: run it with a known stack top within our saved window.
    // It takes (length, address, scratch, scratch) and returns the checksum in `r1`, which
    // the runner only hands out after a successful exit.
    let stack_top = CRC_STUB_ADDR + STUB_WINDOW as u32;
    // The sniffer's output is inverted by the stub (OUT_INV), so undo that to match GDB qCRC.
    let result = cm
        .run_stub_with_stack_result(
            CRC_STUB_ADDR + 4,
            stack_top,
            RESULT_REG,
            len as u32,
            addr,
            CRC_STUB_ADDR,
            CRC_STUB_ADDR,
        )
        .map(|crc| crc ^ 0xffff_ffff);

    // Step 4: stop the DMA, give the target its SRAM back and release the reset.
    cm.write32(resets_reg!(reset), resets | RESETS_RESET_DMA_BITS);
    cm.mem_write(CRC_STUB_ADDR, saved);
    cm.write32(resets_reg!(reset), resets);
    result
}
