//! Idiomatic Rust Flash Subsystem with C-ABI Translation Layer.

pub mod buffer;
pub mod driver;
pub mod engine;
pub mod geometry;
pub mod manager;
pub mod stub;

pub use buffer::{FlashSectorBuffer, MAX_FLASH_BUFFER_SIZE};
pub use driver::{FlashDriver, FlashError, FlashOperation};
pub use engine::*;
pub use geometry::FlashGeometry;
pub use manager::{FlashManager, FlashRegion};
pub use stub::{StubCaps, StubState};

/// Largest write unit the flash manager's static buffer can take that is a whole number of
/// `unit` sized steps: what a driver hands the manager in one `write_block()` call (the C
/// manager sizes its buffer the same way, only dynamically). Units that do not fit are
/// returned unchanged -- the manager rejects them when it sets up the region.
pub fn buffered_write_unit(unit: usize) -> usize {
    if unit == 0 || unit > MAX_FLASH_BUFFER_SIZE {
        return unit;
    }
    unit * (MAX_FLASH_BUFFER_SIZE / unit)
}
