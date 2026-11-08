/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2022 1BitSquared <info@1bitsquared.com>
 * Written by Rafael Silva <perigoso@riseup.net>
 *
 * Rust conversion of blackmagic/src/target/target_flash.c (the buffered write
 * path: flash_buffer_alloc and flash_buffered_flush), made for the Swindle
 * firmware. This file is a translation of that C source into Rust and stays
 * under the same license as the original Black Magic code reproduced below.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice, this
 *    list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * 3. Neither the name of the copyright holder nor the names of its
 *    contributors may be used to endorse or promote products derived from
 *    this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

//! Sector write buffer with dirty window tracking and automatic flush.

use super::driver::{FlashDriver, FlashError};
use super::geometry::FlashGeometry;

pub const MAX_FLASH_BUFFER_SIZE: usize = 2048;

/// Buffer size usable for `geometry`: the driver's preferred write buffer capped by the
/// static array and rounded down to whole write units. A write unit that does not fit in the
/// buffer can never be programmed, so such a geometry is rejected here instead of indexing
/// past the end of the array later.
fn buffer_size(geometry: &FlashGeometry) -> Option<usize> {
    let write_size = geometry.write_size;
    let size = geometry.write_buf_size.min(MAX_FLASH_BUFFER_SIZE);
    if write_size == 0 || size < write_size {
        return None;
    }
    Some(size - size % write_size)
}

/// Aligns `addr` down to a `size` bytes boundary.
#[inline]
fn align_down(addr: u32, size: usize) -> u32 {
    addr & !((size as u32) - 1)
}

/// Manages a sector-sized in-memory buffer to batch sub-sector writes
/// and flush them aligned to `write_size`.
/// Uses a fixed 2048-byte array to ensure 0 dynamic heap allocations during flashing.
pub struct FlashSectorBuffer {
    data: [u8; MAX_FLASH_BUFFER_SIZE],
    base_addr: Option<u32>,
    dirty_low: u32,
    dirty_high: u32,
}

impl Default for FlashSectorBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl FlashSectorBuffer {
    /// Creates a new uninitialized sector buffer (all zeros so it lives in `.bss`;
    /// `write()` fills the buffer with `geometry.erased_value` before loading a window).
    pub const fn new() -> Self {
        Self {
            data: [0; MAX_FLASH_BUFFER_SIZE],
            base_addr: None,
            dirty_low: 0,
            dirty_high: 0,
        }
    }

    /// Clears the buffer and resets base and dirty tracking without freeing memory.
    pub fn clear(&mut self) {
        self.base_addr = None;
        self.dirty_low = 0;
        self.dirty_high = 0;
    }

    /// Checks if the buffer has any uncommitted dirty data.
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.base_addr.is_some() && self.dirty_low < self.dirty_high
    }

    /// Writes data into the sector buffer, flushing and realigning when crossing sector boundaries.
    pub fn write(
        &mut self,
        geometry: &FlashGeometry,
        driver: &mut impl FlashDriver,
        mut dest: u32,
        mut src: &[u8],
    ) -> Result<(), FlashError> {
        let buf_size = buffer_size(geometry).ok_or(FlashError::BufferAllocationFailed)?;

        while !src.is_empty() {
            let base = align_down(dest, buf_size);

            if self.base_addr != Some(base) {
                self.flush(geometry, driver)?;

                // Initialize buffer for new sector with erased byte
                self.data[..buf_size].fill(geometry.erased_byte);
                self.base_addr = Some(base);
                self.dirty_low = u32::MAX;
                self.dirty_high = 0;
            }

            let offset = (dest % (buf_size as u32)) as usize;
            let chunk_len = core::cmp::min(buf_size - offset, src.len());

            self.data[offset..offset + chunk_len].copy_from_slice(&src[..chunk_len]);

            self.dirty_low = core::cmp::min(self.dirty_low, dest);
            self.dirty_high = core::cmp::max(self.dirty_high, dest + chunk_len as u32);

            dest += chunk_len as u32;
            src = &src[chunk_len..];
        }

        Ok(())
    }

    /// Flushes any dirty bytes in the buffer to the underlying driver, aligned to `write_size`.
    pub fn flush(
        &mut self,
        geometry: &FlashGeometry,
        driver: &mut impl FlashDriver,
    ) -> Result<(), FlashError> {
        if !self.is_dirty() {
            return Ok(());
        }

        let base = match self.base_addr {
            Some(b) => b,
            None => return Ok(()),
        };

        // No `prepare()` here: the region prepared `FlashOperation::Write` before it buffered
        // anything (`FlashRegion::set_operation`), and a driver that parks a flashstub in RAM
        // keeps it across the flushes of one session, so a per-flush prepare could only cost
        // round trips (or, before the stub flag existed, a full stub upload).

        let write_size = geometry.write_size;
        // Reject geometries whose write unit does not fit the buffer: the chunks below index
        // `self.data` in `write_size` steps, so they would run past its end.
        buffer_size(geometry).ok_or(FlashError::UnalignedAccess)?;

        let aligned_low = self.dirty_low & !((write_size as u32) - 1);
        let src_start = (aligned_low - base) as usize;
        let total_len = (self.dirty_high - aligned_low) as usize;

        let mut offset = 0;
        while offset < total_len {
            let chunk_addr = aligned_low + offset as u32;
            let chunk_data = &self.data[src_start + offset..src_start + offset + write_size];

            if let Err(e) = driver.write_block(chunk_addr, chunk_data) {
                self.clear();
                return Err(e);
            }

            offset += write_size;
        }

        self.dirty_low = u32::MAX;
        self.dirty_high = 0;
        self.base_addr = None;
        Ok(())
    }
}
