//! Sector write buffer with dirty window tracking and automatic flush.

use super::driver::{FlashDriver, FlashError, FlashOperation};
use super::geometry::FlashGeometry;

pub const MAX_FLASH_BUFFER_SIZE: usize = 2048;

/// Manages a sector-sized in-memory buffer to batch sub-sector writes
/// and flush them aligned to `write_size`.
/// Uses a fixed 2048-byte array to ensure 0 dynamic heap allocations during flashing.
pub struct FlashSectorBuffer {
    data: [u8; MAX_FLASH_BUFFER_SIZE],
    base_addr: Option<u32>,
    dirty_low: u32,
    dirty_high: u32,
}

impl FlashSectorBuffer {
    /// Creates a new uninitialized sector buffer.
    pub const fn new() -> Self {
        Self {
            data: [0xff; MAX_FLASH_BUFFER_SIZE],
            base_addr: None,
            dirty_low: u32::MAX,
            dirty_high: 0,
        }
    }

    /// Clears the buffer and resets base and dirty tracking without freeing memory.
    pub fn clear(&mut self) {
        self.base_addr = None;
        self.dirty_low = u32::MAX;
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
        let buf_size = geometry.write_buf_size.min(MAX_FLASH_BUFFER_SIZE);
        if buf_size == 0 {
            return Err(FlashError::BufferAllocationFailed);
        }

        while !src.is_empty() {
            let base = geometry.align_buffer_down(dest);

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

        driver.prepare(FlashOperation::Write)?;

        let write_size = geometry.write_size;
        if write_size == 0 {
            return Err(FlashError::UnalignedAccess);
        }

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
