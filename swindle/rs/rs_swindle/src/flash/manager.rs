//! High-level FlashManager orchestrating erase, buffered writes, and completion.

use alloc::vec::Vec;
use super::buffer::FlashSectorBuffer;
use super::driver::{FlashDriver, FlashError, FlashOperation};
use super::geometry::FlashGeometry;

/// A registered flash region pairing geometry with its driver backend and sector buffer.
pub struct FlashRegion<D: FlashDriver> {
    pub geometry: FlashGeometry,
    pub extra_erase_sizes: Vec<usize>,
    pub driver: D,
    pub buffer: FlashSectorBuffer,
    pub current_operation: FlashOperation,
}

impl<D: FlashDriver> FlashRegion<D> {
    pub fn new(geometry: FlashGeometry, extra_erase_sizes: Vec<usize>, driver: D) -> Self {
        Self {
            geometry,
            extra_erase_sizes,
            driver,
            buffer: FlashSectorBuffer::new(),
            current_operation: FlashOperation::None,
        }
    }

    /// Sets the driver state to the desired operation, calling `done()` first if needed.
    pub fn set_operation(&mut self, op: FlashOperation) -> Result<(), FlashError> {
        if self.current_operation == op {
            return Ok(());
        }

        let FlashRegion {
            ref geometry,
            ref mut driver,
            ref mut buffer,
            ref mut current_operation,
            ..
        } = *self;

        if *current_operation != FlashOperation::None {
            buffer.flush(geometry, driver)?;
            driver.done()?;
            *current_operation = FlashOperation::None;
        }

        if op != FlashOperation::None {
            driver.prepare(op)?;
            *current_operation = op;
        }
        Ok(())
    }

    /// Buffers and flushes a sub-slice of data.
    pub fn write_chunk(&mut self, dest: u32, data: &[u8]) -> Result<(), FlashError> {
        self.set_operation(FlashOperation::Write)?;
        let FlashRegion {
            ref geometry,
            ref mut driver,
            ref mut buffer,
            ..
        } = *self;
        buffer.write(geometry, driver, dest, data)
    }

    /// Flushes buffer and transitions the driver back to idle (`FlashOperation::None`).
    pub fn finish(&mut self) -> Result<(), FlashError> {
        let mut res = Ok(());
        let FlashRegion {
            ref geometry,
            ref mut driver,
            ref mut buffer,
            ref mut current_operation,
            ..
        } = *self;

        if buffer.is_dirty() {
            if let Err(e) = buffer.flush(geometry, driver) {
                res = Err(e);
            }
        }

        if *current_operation != FlashOperation::None {
            if let Err(e) = driver.done() {
                if res.is_ok() {
                    res = Err(e);
                }
            }
            *current_operation = FlashOperation::None;
        }

        buffer.clear();
        res
    }
}

/// Orchestrates flash operations across one or more flash memory regions.
pub struct FlashManager<D: FlashDriver> {
    pub regions: Vec<FlashRegion<D>>,
    pub active_idx: Option<usize>,
}

impl<D: FlashDriver> FlashManager<D> {
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            active_idx: None,
        }
    }

    pub fn add_region(&mut self, region: FlashRegion<D>) {
        self.regions.push(region);
    }

    pub fn find_region_idx(&self, addr: u32) -> Option<usize> {
        self.regions.iter().position(|r| r.geometry.contains(addr))
    }

    /// Erases a range across flash regions.
    pub fn erase(&mut self, mut addr: u32, mut len: usize) -> Result<(), FlashError> {
        if len == 0 {
            return Ok(());
        }

        let initial_idx = self.find_region_idx(addr).ok_or(FlashError::AddressOutOfBounds)?;
        let mut active_idx = initial_idx;

        while len > 0 {
            let region_idx = self.find_region_idx(addr).ok_or(FlashError::AddressOutOfBounds)?;

            if region_idx != active_idx {
                self.regions[active_idx].finish()?;
                active_idx = region_idx;
            }

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

        self.regions[active_idx].finish()
    }

    /// Writes data buffer into flash, chunking across regions if needed.
    pub fn write(&mut self, mut dest: u32, mut data: &[u8]) -> Result<(), FlashError> {
        if data.is_empty() {
            return Ok(());
        }

        let target_idx = self.find_region_idx(dest).ok_or(FlashError::AddressOutOfBounds)?;

        // Flush any other active region
        if let Some(active) = self.active_idx {
            if active != target_idx {
                self.regions[active].finish()?;
            }
        }
        self.active_idx = Some(target_idx);

        while !data.is_empty() {
            let region_idx = self.find_region_idx(dest).ok_or(FlashError::AddressOutOfBounds)?;

            if Some(region_idx) != self.active_idx {
                if let Some(active) = self.active_idx {
                    self.regions[active].finish()?;
                }
                self.active_idx = Some(region_idx);
            }

            let chunk_end = {
                let region = &self.regions[region_idx];
                core::cmp::min(dest + data.len() as u32, region.geometry.end())
            };
            let chunk_len = (chunk_end - dest) as usize;

            let region = &mut self.regions[region_idx];
            region.write_chunk(dest, &data[..chunk_len])?;

            dest = chunk_end;
            data = &data[chunk_len..];
        }

        Ok(())
    }

    /// Finalizes all regions: flushes dirty sector buffers and sets drivers to idle.
    pub fn complete(&mut self) -> Result<(), FlashError> {
        let mut first_err = Ok(());
        for region in &mut self.regions {
            if let Err(e) = region.finish() {
                if first_err.is_ok() {
                    first_err = Err(e);
                }
            }
        }
        self.active_idx = None;
        first_err
    }

    /// Performs mass erase on all registered regions.
    pub fn mass_erase_all<P: FnMut()>(&mut self, mut progress_cb: P) -> Result<(), FlashError> {
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
            region.finish()?;
        }
        Ok(())
    }
}
