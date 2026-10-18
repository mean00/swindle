//! Typed memory accessors and memory region descriptions (32-bit target addresses).

use super::driver::{TargetDriver, TargetError};

/// Representation of a RAM region in target memory space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RamRegion {
    pub start: u32,
    pub length: u32,
}

impl RamRegion {
    pub const fn new(start: u32, length: u32) -> Self {
        Self { start, length }
    }

    pub const fn end(&self) -> u32 {
        self.start.saturating_add(self.length)
    }

    pub const fn contains(&self, addr: u32) -> bool {
        addr >= self.start && addr < self.end()
    }
}

/// Read a 32-bit little-endian word from target memory.
pub fn read_u32<D: TargetDriver + ?Sized>(driver: &D, addr: u32) -> Result<u32, TargetError> {
    let mut buf = [0u8; 4];
    driver.mem_read(&mut buf, addr)?;
    Ok(u32::from_le_bytes(buf))
}

/// Write a 32-bit little-endian word to target memory.
pub fn write_u32<D: TargetDriver + ?Sized>(driver: &D, addr: u32, val: u32) -> Result<(), TargetError> {
    driver.mem_write(addr, &val.to_le_bytes())
}

/// Read a 16-bit little-endian halfword from target memory.
pub fn read_u16<D: TargetDriver + ?Sized>(driver: &D, addr: u32) -> Result<u16, TargetError> {
    let mut buf = [0u8; 2];
    driver.mem_read(&mut buf, addr)?;
    Ok(u16::from_le_bytes(buf))
}

/// Write a 16-bit little-endian halfword to target memory.
pub fn write_u16<D: TargetDriver + ?Sized>(driver: &D, addr: u32, val: u16) -> Result<(), TargetError> {
    driver.mem_write(addr, &val.to_le_bytes())
}

/// Read an 8-bit byte from target memory.
pub fn read_u8<D: TargetDriver + ?Sized>(driver: &D, addr: u32) -> Result<u8, TargetError> {
    let mut buf = [0u8; 1];
    driver.mem_read(&mut buf, addr)?;
    Ok(buf[0])
}

/// Write an 8-bit byte to target memory.
pub fn write_u8<D: TargetDriver + ?Sized>(driver: &D, addr: u32, val: u8) -> Result<(), TargetError> {
    driver.mem_write(addr, &[val])
}
