/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2022 1BitSquared <info@1bitsquared.com>
 * Written by Rafael Silva <perigoso@riseup.net>
 *
 * Rust conversion of blackmagic/src/target/target_flash.c and
 * blackmagic/src/target/target_internal.h (flash geometry and block size
 * selection), made for the Swindle firmware. This file is a translation of
 * those C sources into Rust and stays under the same license as the original
 * Black Magic code reproduced below.
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

//! Flash memory geometry calculations and alignment logic.

/// Geometry description for a contiguous flash memory region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlashGeometry {
    pub start: u32,
    pub length: usize,
    pub block_size: usize,
    pub write_size: usize,
    pub write_buf_size: usize,
    pub erased_byte: u8,
}

impl FlashGeometry {
    pub fn new(
        start: u32,
        length: usize,
        block_size: usize,
        write_size: usize,
        write_buf_size: usize,
        erased_byte: u8,
    ) -> Self {
        let actual_block_size = if block_size == 0 { 1024 } else { block_size };
        let actual_write_size = if write_size == 0 { actual_block_size } else { write_size };
        let actual_buf_size = if write_buf_size == 0 { actual_write_size } else { write_buf_size };
        Self {
            start,
            length,
            block_size: actual_block_size,
            write_size: actual_write_size,
            write_buf_size: actual_buf_size,
            erased_byte,
        }
    }

    /// End address (exclusive) of the flash region.
    #[inline]
    pub fn end(&self) -> u32 {
        self.start.wrapping_add(self.length as u32)
    }

    /// Checks if a given address falls inside this flash region.
    #[inline]
    pub fn contains(&self, addr: u32) -> bool {
        addr >= self.start && addr < self.end()
    }

    /// Aligns address down to the block size boundary.
    #[inline]
    pub fn align_block_down(&self, addr: u32) -> u32 {
        addr & !((self.block_size as u32) - 1)
    }

    /// Aligns address down to the write size boundary.
    #[inline]
    pub fn align_write_down(&self, addr: u32) -> u32 {
        addr & !((self.write_size as u32) - 1)
    }

    /// Aligns address down to the write buffer boundary.
    #[inline]
    pub fn align_buffer_down(&self, addr: u32) -> u32 {
        addr & !((self.write_buf_size as u32) - 1)
    }

    /// Selects the best erase block size covering `addr` from available erase sizes.
    pub fn best_erase_block(&self, addr: u32, len: usize, extra_sizes: &[usize]) -> usize {
        let local_start = self.align_block_down(addr);
        let padded_len = len + (addr.wrapping_sub(local_start)) as usize;
        let mut best = self.block_size;

        for &size in extra_sizes {
            if size > 0 && (local_start & ((size as u32) - 1)) == 0 && padded_len >= size && size > best {
                best = size;
            }
        }
        best
    }
}
