//! Read-ahead line cache for GDB `m` (memory read) packets.
//!
//! GDB's `x` command reads memory one element at a time, issuing one
//! `$m addr,size` packet per element (`x /64bx` = 64 one-byte reads, and
//! newer GDB builds also poll a few bytes at a time). Without a cache each of
//! those packets costs a full BMP memory-read call (multiple SWD transactions).
//!
//! This module lets the stub serve consecutive small reads from a single
//! 16-byte line fetched from the target once, cutting SWD traffic ~8x for `bx`
//! dumps (≈96 tx → ≈12 tx per 16 bytes) and ~2x for `wx` dumps.
//!
//! ## Safety rules (correctness is the hard requirement)
//!
//! - Only addresses inside RAM/flash map regions are ever cached. Peripheral
//!   space is never cached, because reads there can have side effects and the
//!   map can't be trusted to stay stable.
//! - The cache is invalidated on every target resume/step/reset, on every
//!   memory write (see `commands/run.rs`, `commands/memory.rs`,
//!   `commands/flash.rs` and the `bmp` write wrappers), and on every attach /
//!   SWD scan / detach (a different target may own the same address space).
//!   It can therefore never serve stale data across execution, modification,
//!   or target switches.
//!
//! The stub is single-threaded (one GDB command at a time), so a `static mut`
//! line is safe.

use crate::bmp;
use crate::bmp::mapping::{Flash, Ram};

/// Cache line size in bytes (matches a 16-byte SWD burst).
pub const LINE_SIZE: usize = 16;

/// Line-alignment mask: clears the low `log2(LINE_SIZE)` bits, rounding an
/// address down to its line base. Kept in sync with [`LINE_SIZE`] — the cache
/// is 16 B, so this is `!0xF` (`0xFFFFFFF0`).
pub const ALIGN_MASK: u32 = !((LINE_SIZE - 1) as u32);

/// GDB `$m` transfer chunk — max bytes fetched per C read call in the `_m`
/// handler. Always a multiple of [`LINE_SIZE`] so chunked reads stay aligned
/// to cache lines.
pub const READ_CHUNK_SIZE: usize = LINE_SIZE * 4;

/// A single cached line.
struct Line {
    valid: bool,
    /// 16-byte aligned base address of the cached line.
    base: u32,
    data: [u8; LINE_SIZE],
}

static mut CACHE: Line = Line { valid: false, base: 0, data: [0; LINE_SIZE] };

/// True when `[addr, addr + len)` lies entirely inside a RAM or flash map
/// region. Performs one map query for RAM and one for flash (both hit the
/// C-side target descriptor — cheap, and only on cache misses).
pub fn cacheable(addr: u32, len: u32) -> bool {
    let start = addr as u64;
    let end = (addr as u64).saturating_add(len as u64);
    for block in bmp::bmp_get_mapping(Ram) {
        let b_start = block.start_address as u64;
        let b_end = (block.start_address as u64).saturating_add(block.length as u64);
        if start >= b_start && end <= b_end {
            return true;
        }
    }
    for block in bmp::bmp_get_mapping(Flash) {
        let b_start = block.start_address as u64;
        let b_end = (block.start_address as u64).saturating_add(block.length as u64);
        if start >= b_start && end <= b_end {
            return true;
        }
    }
    false
}

/// Invalidate the cached line. Call whenever the target may have modified
/// memory: resume/step/reset, or any memory write.
pub fn invalidate() {
    unsafe {
        CACHE.valid = false;
    }
}

/// Try to serve `n` bytes at `addr` from the cache.
/// Returns `true` and fills `out` when the whole range is cached.
pub fn try_read(addr: u32, n: usize, out: &mut [u8]) -> bool {
    if n == 0 || n > LINE_SIZE {
        return false;
    }
    unsafe {
        let c = &CACHE;
        // base == (addr & ALIGN_MASK) guarantees addr >= base, so the offset
        // math below can't underflow.
        if !c.valid || c.base != (addr & ALIGN_MASK) {
            return false;
        }
        let off = (addr - c.base) as usize;
        if off + n > LINE_SIZE {
            return false;
        }
        out[..n].copy_from_slice(&c.data[off..off + n]);
        true
    }
}

/// Store a full 16-byte line read from `base`.
///
/// `base` must be 16-byte aligned and inside a RAM/flash region (the caller
/// verifies via [`cacheable`]).
pub fn fill(base: u32, data: &[u8]) {
    if data.len() != LINE_SIZE {
        return;
    }
    unsafe {
        CACHE.base = base & ALIGN_MASK;
        CACHE.data.copy_from_slice(data);
        CACHE.valid = true;
    }
}
