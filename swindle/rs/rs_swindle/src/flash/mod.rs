//! Idiomatic Rust Flash Subsystem with C-ABI Translation Layer.

pub mod buffer;
pub mod c_adapter;
pub mod driver;
pub mod geometry;
pub mod manager;

pub use buffer::FlashSectorBuffer;
pub use c_adapter::*;
pub use driver::{FlashDriver, FlashError, FlashOperation};
pub use geometry::FlashGeometry;
pub use manager::{FlashManager, FlashRegion};
