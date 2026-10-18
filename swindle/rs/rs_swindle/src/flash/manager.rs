//! High-level FlashManager orchestrating erase, buffered writes, and completion.

use super::buffer::FlashSectorBuffer;
use super::driver::{FlashDriver, FlashError, FlashOperation};
use super::geometry::FlashGeometry;

/// Maximum number of flash memory regions a single target can register.
pub const MAX_FLASH_REGIONS: usize = 4;

/// Maximum number of extra erase block sizes per flash region.
pub const MAX_EXTRA_ERASE_SIZES: usize = 8;

/// A registered flash region pairing geometry with its driver backend.
pub struct FlashRegion<D: FlashDriver> {
    pub geometry: FlashGeometry,
    pub extra_erase_sizes: heapless::Vec<usize, MAX_EXTRA_ERASE_SIZES>,
    pub driver: D,
    pub current_operation: FlashOperation,
}

impl<D: FlashDriver> FlashRegion<D> {
    pub fn new(
        geometry: FlashGeometry,
        extra_erase_sizes: heapless::Vec<usize, MAX_EXTRA_ERASE_SIZES>,
        driver: D,
    ) -> Self {
        Self {
            geometry,
            extra_erase_sizes,
            driver,
            current_operation: FlashOperation::None,
        }
    }

    /// Sets the driver state to the desired operation, calling `done()` first if needed.
    pub fn set_operation(&mut self, op: FlashOperation) -> Result<(), FlashError> {
        if self.current_operation == op {
            return Ok(());
        }

        if self.current_operation != FlashOperation::None {
            self.driver.done()?;
            self.current_operation = FlashOperation::None;
        }

        if op != FlashOperation::None {
            self.driver.prepare(op)?;
            self.current_operation = op;
        }
        Ok(())
    }
}

/// Orchestrates flash operations across one or more flash memory regions.
/// Uses zero heap allocations for bookkeeping (`heapless::Vec` for regions and erase sizes).
pub struct FlashManager<D: FlashDriver> {
    pub regions: heapless::Vec<FlashRegion<D>, MAX_FLASH_REGIONS>,
    pub active_idx: Option<usize>,
    pub buffer: FlashSectorBuffer,
}

impl<D: FlashDriver> FlashManager<D> {
    pub fn new() -> Self {
        Self {
            regions: heapless::Vec::new(),
            active_idx: None,
            buffer: FlashSectorBuffer::new(),
        }
    }

    pub fn add_region(&mut self, region: FlashRegion<D>) {
        let _ = self.regions.push(region);
    }

    pub fn find_region_idx(&self, addr: u32) -> Option<usize> {
        self.regions.iter().position(|r| r.geometry.contains(addr))
    }

    /// Flushes any dirty data in the sector buffer to the currently active region.
    pub fn flush_active(&mut self) -> Result<(), FlashError> {
        if let Some(idx) = self.active_idx {
            let region = &mut self.regions[idx];
            if self.buffer.is_dirty() {
                self.buffer.flush(&region.geometry, &mut region.driver)?;
            }
        }
        Ok(())
    }

    /// Flushes any dirty data in the buffer and transitions the active region driver back to idle.
    pub fn finish_active(&mut self) -> Result<(), FlashError> {
        let mut res = Ok(());
        if let Some(idx) = self.active_idx {
            let region = &mut self.regions[idx];
            if self.buffer.is_dirty() {
                if let Err(e) = self.buffer.flush(&region.geometry, &mut region.driver) {
                    res = Err(e);
                }
            }
            if region.current_operation != FlashOperation::None {
                if let Err(e) = region.driver.done() {
                    if res.is_ok() {
                        res = Err(e);
                    }
                }
                region.current_operation = FlashOperation::None;
            }
            self.buffer.clear();
            self.active_idx = None;
        }
        res
    }

    /// Erases a range across flash regions.
    pub fn erase(&mut self, mut addr: u32, mut len: usize) -> Result<(), FlashError> {
        if len == 0 {
            return Ok(());
        }

        let initial_idx = self.find_region_idx(addr).ok_or(FlashError::AddressOutOfBounds)?;
        if self.active_idx != Some(initial_idx) {
            self.finish_active()?;
            self.active_idx = Some(initial_idx);
        }

        while len > 0 {
            let region_idx = self.find_region_idx(addr).ok_or(FlashError::AddressOutOfBounds)?;

            if Some(region_idx) != self.active_idx {
                self.finish_active()?;
                self.active_idx = Some(region_idx);
            }

            let active_idx = region_idx;
            let (local_start, best_size, local_end) = {
                let region = &self.regions[active_idx];
                let geom = &region.geometry;
                let start = geom.align_block_down(addr);
                let size = geom.best_erase_block(addr, len, &region.extra_erase_sizes);
                let end = start + size as u32;
                (start, size, end)
            };

            let region = &mut self.regions[active_idx];
            region.set_operation(FlashOperation::Erase)?;
            region.driver.erase_block(local_start, best_size)?;

            let advance = core::cmp::min((local_end - addr) as usize, len);
            len -= advance;
            addr = local_end;
        }

        self.finish_active()
    }

    /// Writes data buffer into flash, chunking across regions if needed.
    pub fn write(&mut self, mut dest: u32, mut data: &[u8]) -> Result<(), FlashError> {
        if data.is_empty() {
            return Ok(());
        }

        let target_idx = self.find_region_idx(dest).ok_or(FlashError::AddressOutOfBounds)?;

        // If crossing region boundary, flush previous region
        if self.active_idx != Some(target_idx) {
            self.finish_active()?;
            self.active_idx = Some(target_idx);
        }

        while !data.is_empty() {
            let region_idx = self.find_region_idx(dest).ok_or(FlashError::AddressOutOfBounds)?;

            if Some(region_idx) != self.active_idx {
                self.finish_active()?;
                self.active_idx = Some(region_idx);
            }

            let active_idx = region_idx;
            let chunk_end = {
                let region = &self.regions[active_idx];
                core::cmp::min(dest + data.len() as u32, region.geometry.end())
            };
            let chunk_len = (chunk_end - dest) as usize;

            let region = &mut self.regions[active_idx];
            region.set_operation(FlashOperation::Write)?;
            self.buffer.write(
                &region.geometry,
                &mut region.driver,
                dest,
                &data[..chunk_len],
            )?;

            dest = chunk_end;
            data = &data[chunk_len..];
        }

        Ok(())
    }

    /// Finalizes all regions: flushes dirty sector buffers and sets drivers to idle.
    pub fn complete(&mut self) -> Result<(), FlashError> {
        let mut first_err = self.finish_active();
        for region in &mut self.regions {
            if region.current_operation != FlashOperation::None {
                if let Err(e) = region.driver.done() {
                    if first_err.is_ok() {
                        first_err = Err(e);
                    }
                }
                region.current_operation = FlashOperation::None;
            }
        }
        first_err
    }

    /// Performs mass erase on all registered regions.
    pub fn mass_erase_all<P: FnMut()>(&mut self, mut progress_cb: P) -> Result<(), FlashError> {
        self.finish_active()?;
        for region in &mut self.regions {
            let has_mass = region.driver.has_mass_erase();
            let op = if has_mass { FlashOperation::MassErase } else { FlashOperation::Erase };
            region.set_operation(op)?;

            if has_mass {
                region.driver.mass_erase()?;
                progress_cb();
            } else {
                let block = region.geometry.block_size as u32;
                let mut addr = region.geometry.start;
                let end = region.geometry.end();
                while addr < end {
                    region.driver.erase_block(addr, region.geometry.block_size)?;
                    progress_cb();
                    addr = addr.wrapping_add(block);
                }
            }
            region.set_operation(FlashOperation::None)?;
        }
        Ok(())
    }
}
