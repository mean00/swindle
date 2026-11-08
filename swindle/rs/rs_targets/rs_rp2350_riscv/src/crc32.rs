//! CRC32 through the DMA sniffer, so GDB's `qCRC` never has to read the range.
//!
//! The Hazard3 half of `rp2350_crc32()` from `blackmagic/src/target/rp2350.c`. The stub is
//! `rp2350_crc32_riscv`: it programs DMA channel 0 to stream the range through
//! the sniffing hardware and retires with the result in `a2`. The SRAM it overwrites belongs
//! to the target, so it is saved and put back through the probe's shared scratch buffer.
//!
//! The Arm cores of the same chip use a different stub and register, so that half lives in
//! `rs_rp2350_arm::crc32`.

use rs_rp2350_common::flashstub::{RP2350_CRC32_RISCV_BIN, RP2350_CRC32_RISCV_BIN_LEN};
use rs_rp2350_common::registers::*;
use rs_rp_common::access::{FlashAccess, HartAccess};
use rsbmp::riscv::RiscvHart;
use rsbmp::target::temporary_buffer;

/// Below this the software CRC is cheaper than a stub round trip (C `RP2350_CRC_MIN_SIZE`).
const MIN_SIZE: usize = 300;

/// Stack allocated within the saved SRAM window for the stub.
const STUB_STACK_SIZE: usize = 64;

/// Words of SRAM the stub works in: itself, plus the word the DMA writes through,
/// plus a small stack window.
const STUB_WINDOW: usize = (RP2350_CRC32_RISCV_BIN_LEN + 4 + STUB_STACK_SIZE + 3) & !3;

// The stub, the DMA's scratch word and the stack it runs on have to stay below the
// flashstub stack region, or a CRC run would walk over the flash writer's workspace.
const _: () = assert!(CRC_STUB_ADDR + STUB_WINDOW as u32 <= STUB_STACK_LOCATION);

/// Fast CRC32 of `[addr, addr + len)` (C `rp2350_crc32()` on a Hazard3 hart).
///
/// `None` means "not this range": too short, misaligned, overlapping the stub's own
/// scratch window, or the stub itself failed. The caller then falls back to the generic
/// debug-link CRC.
pub fn crc32(hart: &mut RiscvHart, addr: u32, len: usize) -> Option<u32> {
    if len < MIN_SIZE || addr & 3 != 0 || len & 3 != 0 {
        return None;
    }
    // A range that covers the scratch window would checksum the stub's own code.
    if addr < CRC_STUB_ADDR + STUB_WINDOW as u32 && addr.wrapping_add(len as u32) > CRC_STUB_ADDR {
        return None;
    }

    let saved = temporary_buffer(STUB_WINDOW)?;

    // Pulse the DMA out of reset for the transfer, restoring the state afterwards.
    let resets = {
        let m = HartAccess::new(hart);
        let resets = m.read32(resets_reg!(reset));
        let dma_released = resets & !RESETS_RESET_DMA_BITS;
        m.write32(resets_reg!(reset), dma_released | RESETS_RESET_DMA_BITS);

        // Step 1: SRAM belongs to the target, so keep what the stub overwrites.
        m.mem_read(saved, CRC_STUB_ADDR);
        m.write32(resets_reg!(reset), dma_released);
        while m.read32(resets_reg!(reset_done)) & RESETS_RESET_DMA_BITS == 0 {}
        // Step 2: upload the stub just past the word the DMA writes into.
        m.mem_write(CRC_STUB_ADDR + 4, RP2350_CRC32_RISCV_BIN);
        resets
    };

    // Step 3: run it with a known stack top within our saved window.
    // It takes (length, address, scratch, scratch) and leaves the checksum in `a2`; the
    // runner reports it only once the stub published the standard exit code in `a0`.
    let stack_top = CRC_STUB_ADDR + STUB_WINDOW as u32;
    let result = hart
        .run_stub_with_stack_ret(
            CRC_STUB_ADDR + 4,
            stack_top,
            len as u32,
            addr,
            CRC_STUB_ADDR,
            CRC_STUB_ADDR,
        )
        // The sniffer's output is inverted by the stub (OUT_INV), so undo that to match GDB qCRC.
        .map(|crc| crc ^ 0xffff_ffff);

    // Step 4: stop the DMA, give the target its SRAM back and release the reset.
    let m = HartAccess::new(hart);
    m.write32(resets_reg!(reset), resets | RESETS_RESET_DMA_BITS);
    m.mem_write(CRC_STUB_ADDR, saved);
    m.write32(resets_reg!(reset), resets);
    result
}
