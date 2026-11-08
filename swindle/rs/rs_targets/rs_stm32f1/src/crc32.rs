//! Hardware CRC32 through the CRC unit of the GD32 parts.
//!
//! This is the Rust counterpart of the C `gd32_crc32()` (`stm32f1.c`): for a range big
//! enough to be worth it, the probe loads a small Thumb stub into the target's SRAM, has
//! the target stream the range through its own CRC unit, then puts the SRAM back. Only
//! the stub and its saved copy cross the debug link, which is what makes GDB's `qCRC`
//! packet (and so `compare-sections`) fast on these parts.

use crate::flashstub_crc32::{GD32_CRC32_BIN, GD32_CRC32_BIN_LEN};
use rsbmp::target::CortexMAccess;
use rsbmp::target::cortexm_access::temporary_buffer;

/// Where the stub is loaded: the start of the main SRAM on every part this driver probes.
const STUB_ADDR: u32 = 0x2000_0000;

/// Register the stub leaves its result in (`r1`).
const RESULT_REG: u32 = 1;

/// Shortest range worth a stub round trip; below this the plain debug-link CRC is faster
/// (the C driver's threshold).
const MIN_LEN: usize = 300;

/// CRC32 of `[addr, addr + len)` computed by the target itself.
///
/// Returns `None` -- and the caller then falls back to the generic debug-link CRC -- for
/// the same ranges the C driver refuses: too short, not word-aligned, or overlapping the
/// SRAM the stub has to occupy.
pub fn gd32_crc32(cm: &mut CortexMAccess, addr: u32, len: usize) -> Option<u32> {
    if len < MIN_LEN || addr & 3 != 0 || len & 3 != 0 {
        return None;
    }

    let stub_start = STUB_ADDR as usize;
    let stub_end = stub_start + GD32_CRC32_BIN_LEN;
    // The stub must not overlap the range being checksummed.
    if addr as usize >= stub_start && (addr as usize) < stub_end {
        return None;
    }
    if (addr as usize) <= stub_start && (addr as usize) + len > stub_start {
        return None;
    }

    // Stash the SRAM the stub is about to overwrite, load it, run it, put the SRAM back.
    let saved = temporary_buffer(GD32_CRC32_BIN_LEN)?;
    cm.mem_read(saved, STUB_ADDR);
    cm.mem_write(STUB_ADDR, &GD32_CRC32_BIN);

    // The stub publishes the CRC in r1 and the standard stub exit code; the target streams
    // `len / 4` words, which is why unaligned ranges were refused above.
    let crc = cm.run_stub_result(STUB_ADDR + 1, RESULT_REG, addr, (len >> 2) as u32, 0, 0);

    cm.mem_write(STUB_ADDR, saved);
    crc
}
