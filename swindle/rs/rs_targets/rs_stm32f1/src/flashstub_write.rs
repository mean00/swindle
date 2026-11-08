//! Running the FPEC flash-programming stub on the target.
//!
//! The write-side counterpart of [`crate::crc32`]: programming a chunk through the debug
//! link costs one bus-stalled program step per halfword/word (`CortexMAccess::write_aligned()`),
//! so instead the probe drops the whole chunk in the target's SRAM, runs the stub next to it
//! and lets the target program its own flash at core speed. One bulk memory write carries
//! the chunk, however many program steps it takes to flush it.
//!
//! Two stub images exist, differing only in the size of a program step: [16-bit](crate::flashstub_write16)
//! for the STM32F0/F1/F3 parts and [32-bit](crate::flashstub_write32) for the GD32/AT32
//! clones that accept 32-bit steps.
//!
//! Unlike the CRC stub, the SRAM the write stub and its chunk occupy is not saved and put
//! back: the chunk is as large as what the flash manager writes per call, so stashing it
//! would double exactly the traffic this path exists to avoid. The C drivers' flash stubs
//! (`efm32`, `lmi`, `rp2040`) work the same way -- a `load` rewrites that part of RAM anyway.
//!
//! **Not on the write path** ([`crate::flash::write`] programs the debug link directly): the
//! FPEC needs ~50 us per program step however the step is issued, so the stub only wins when
//! the link is the slower of the two, and it loses badly when the target core is slow -- the
//! probe resets the target before programming, so the stub usually runs on the 8 MHz clock
//! the part comes out of reset with. Measured with 32 KiB on a GD32F303: 1322 ms via the stub
//! against 1322 ms over the link with the target running at 96 MHz, and 2575 ms against
//! 1350 ms on the reset clock. Kept for parts whose program step is much shorter than one
//! link byte time (the 128-bit steps of the F4, say), where the link really is the limit.

use crate::flash::{FpecLayout, WRITE_CHUNK};
use crate::flashstub_write16::{STM32F1_WRITE_BIN, STM32F1_WRITE_BIN_LEN};
use crate::flashstub_write32::{GD32_WRITE_BIN, GD32_WRITE_BIN_LEN};
use crate::registers::{FPEC_STATUS, SRAM_BASE};
use rsbmp::target::{ALIGN_16BIT, ALIGN_32BIT, CortexMAccess};

/// Where the stub is loaded: the start of the main SRAM on every part this driver probes,
/// moved up one window so it cannot collide with the CRC stub, which owns the base of SRAM
/// (see [`crate::crc32`]).
///
/// This path is not on the write path today (see the module docs): [`crate::flash::write`]
/// programs the debug link directly, so the move is unverified on hardware and the image is
/// kept only for parts whose program step is much shorter than a link byte time.
const STUB_ADDR: u32 = SRAM_BASE + 0x800;

/// Smallest SRAM of any part in the family (the F103x4 has 6 KiB), which is the budget the
/// stub image and the chunk behind it have to fit in.
const SRAM_MIN_SIZE: u32 = 6 * 1024;

// The chunk lands right behind the image: both have to stay inside the smallest SRAM of the
// family, and above the CRC stub's window at the base.
const _: () = assert!(
    STUB_ADDR + align_up(STM32F1_WRITE_BIN_LEN as u32, 4) + WRITE_CHUNK as u32 <= SRAM_BASE + SRAM_MIN_SIZE
);
const _: () = assert!(
    STUB_ADDR + align_up(GD32_WRITE_BIN_LEN as u32, 4) + WRITE_CHUNK as u32 <= SRAM_BASE + SRAM_MIN_SIZE
);

/// Bytes per program step of `step`-sized parts, and the stub image that performs it.
#[derive(Clone, Copy)]
enum Stub {
    /// 16-bit program steps, STM32F0/F1/F3.
    Halfword,
    /// 32-bit program steps, GD32/AT32 clones.
    Word,
}

impl Stub {
    /// The stub for parts programmed with `align` (a `FpecLayout::align()` value), if one
    /// exists for that bus width.
    fn for_align(align: u8) -> Option<Self> {
        match align {
            ALIGN_16BIT => Some(Self::Halfword),
            ALIGN_32BIT => Some(Self::Word),
            _ => None,
        }
    }

    /// Image of the stub.
    fn image(self) -> &'static [u8] {
        match self {
            Self::Halfword => &STM32F1_WRITE_BIN,
            Self::Word => &GD32_WRITE_BIN,
        }
    }

    /// Size of one program step, in bytes: what the destination, the length and the SRAM
    /// buffer all have to be a multiple of.
    fn step(self) -> u32 {
        match self {
            Self::Halfword => 2,
            Self::Word => 4,
        }
    }
}

/// `value` rounded up to the next multiple of `align` (a power of two).
const fn align_up(value: u32, align: u32) -> u32 {
    (value + align - 1) & !(align - 1)
}

/// Program `data` into the flash at `dest` with the stub of `layout`'s part.
///
/// Returns `None` when the range is outside what the stub does -- a bus width it has no
/// image for, an address or length that is not a whole number of program steps, or a chunk
/// longer than the flash manager's write chunk -- and the caller then writes it over the
/// debug link instead. `Some(false)` means the stub ran but the flash controller of the
/// bank holding `dest` reported an error.
///
/// The caller has already unlocked that bank and put it in programming mode: the stub only
/// issues the program steps and watches their status.
pub fn program(cm: &mut CortexMAccess, layout: &FpecLayout, dest: u32, data: &[u8]) -> Option<bool> {
    let stub = Stub::for_align(layout.align())?;
    let step = stub.step();
    if data.is_empty() ||
        data.len() > WRITE_CHUNK ||
        dest % step != 0 ||
        data.len() % step as usize != 0
    {
        return None;
    }

    // The chunk goes right behind the stub, on a word boundary so the 32-bit flavour can
    // load a word at a time; the stub image is short enough that this stays far inside the
    // SRAM of every part this crate probes.
    let image = stub.image();
    let buffer_addr = STUB_ADDR + align_up(image.len() as u32, step);
    cm.mem_write(STUB_ADDR, image);
    cm.mem_write(buffer_addr, data);

    // `run_stub()` is `true` once the whole chunk is in flash and `false` when the stub gave
    // up on a flash error (it traps with a non-zero immediate then).
    let status_addr = FPEC_STATUS + layout.bank_offset_for(dest);
    let ok = cm.run_stub(STUB_ADDR + 1, dest, buffer_addr, data.len() as u32, status_addr);
    Some(ok)
}
