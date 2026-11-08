//! The target-side operations the RP flash support needs, whatever core drives the target.
//!
//! The SPI flash layer ([`crate::spi`], [`crate::sfdp`]) and the two RP flash controllers
//! (the RP2040's SSI and the RP2350's QMI) only ever touch the target through this trait:
//! read or write a word, copy a buffer in or out, ask whether the link faulted, and restart
//! the core once the flash is back in place. That is all a flash controller needs, and it
//! is the same whether the chip underneath is a Cortex-M core reached through
//! [`CortexMAccess`] or an RP2350 Hazard3 core reached through [`HartAccess`].
//!
//! The C drivers had this seam too, but implicitly: `target_mem32_read32()` and friends
//! dispatch on the target's architecture at run time. Here it is a trait, so the shared
//! flash code is compiled once and each core type brings its own implementation.

use core::cell::{Cell, RefCell};

use rsbmp::riscv::RiscvHart;
use rsbmp::target::CortexMAccess;

/// What a flash controller needs from the target it is driving.
///
/// The methods mirror the C `target_mem32_*` family a driver's `target_s` hooks go
/// through, including their error convention: a failed access is recorded and reported by
/// [`Self::check_error`] rather than returned, which is what lets the erase/program loops
/// stay the unbounded loops of the C driver and still give up on a faulted bus.
pub trait FlashAccess {
    /// Read a 32-bit word, without checking for a bus fault (C `target_mem32_read32`).
    fn read32(&self, addr: u32) -> u32;

    /// Write a 32-bit word (C `target_mem32_write32`).
    fn write32(&self, addr: u32, value: u32);

    /// Read `dst.len()` bytes at `src` (C `target_mem32_read`).
    fn mem_read(&self, dst: &mut [u8], src: u32);

    /// Write `src` at `dst` (C `target_mem32_write`).
    fn mem_write(&self, dst: u32, src: &[u8]);

    /// Whether an access since the last clear faulted.
    fn check_error(&self) -> bool;

    /// Reset the core, so that it restarts from the flash being handed back to it
    /// (C `target_reset`).
    fn reset(&self);
}

impl FlashAccess for CortexMAccess {
    fn read32(&self, addr: u32) -> u32 {
        CortexMAccess::read32(self, addr)
    }

    fn write32(&self, addr: u32, value: u32) {
        CortexMAccess::write32(self, addr, value);
    }

    fn mem_read(&self, dst: &mut [u8], src: u32) {
        CortexMAccess::mem_read(self, dst, src);
    }

    fn mem_write(&self, dst: u32, src: &[u8]) {
        CortexMAccess::mem_write(self, dst, src);
    }

    fn check_error(&self) -> bool {
        CortexMAccess::check_error(self)
    }

    fn reset(&self) {
        CortexMAccess::reset(self);
    }
}

/// A RISC-V hart seen through [`FlashAccess`].
///
/// [`RiscvHart`] hands out target memory through `&mut` (its access dispatcher may switch
/// methods as it goes), while the flash layer takes the target by shared reference, the way
/// the C callbacks do. The cell is that difference and nothing else: the trait's methods are
/// never re-entered, so the borrow is taken and dropped per access.
///
/// A failed access becomes the sticky error [`FlashAccess::check_error`] reports, the same
/// way a faulted Cortex-M bus access does.
pub struct HartAccess<'a> {
    /// The hart to reach the target through.
    hart: RefCell<&'a mut RiscvHart>,
    /// Whether an access failed since the object was made.
    failed: Cell<bool>,
}

impl<'a> HartAccess<'a> {
    /// Reach the target `hart` drives.
    pub fn new(hart: &'a mut RiscvHart) -> Self {
        Self {
            hart: RefCell::new(hart),
            failed: Cell::new(false),
        }
    }

    /// Record the outcome of one access.
    fn record(&self, succeeded: bool) {
        if !succeeded {
            self.failed.set(true);
        }
    }
}

impl FlashAccess for HartAccess<'_> {
    fn read32(&self, addr: u32) -> u32 {
        let mut buffer = [0u8; 4];
        self.mem_read(&mut buffer, addr);
        u32::from_le_bytes(buffer)
    }

    fn write32(&self, addr: u32, value: u32) {
        self.mem_write(addr, &value.to_le_bytes());
    }

    fn mem_read(&self, dst: &mut [u8], src: u32) {
        let succeeded = self.hart.borrow_mut().mem_read(src, dst).is_ok();
        self.record(succeeded);
    }

    fn mem_write(&self, dst: u32, src: &[u8]) {
        let succeeded = self.hart.borrow_mut().mem_write(dst, src).is_ok();
        self.record(succeeded);
    }

    fn check_error(&self) -> bool {
        self.failed.get()
    }

    fn reset(&self) {
        let _ = self.hart.borrow_mut().reset();
    }
}
