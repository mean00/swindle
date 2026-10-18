//! Swindle Rust Target Subsystem.
//!
//! Provides idiomatic Rust target abstractions (`TargetDriver`, `TargetRegistry`, `RamRegion`),
//! along with a C-ABI translation layer (`c_adapter`) that exposes the 35+ C symbols originally
//! implemented by `blackmagic/src/target/target.c`.

pub mod driver;
pub mod memory;
pub mod registry;
pub mod c_adapter;

pub use driver::{BreakwatchType, HaltReason, TargetDriver, TargetError};
pub use memory::{read_u16, read_u32, read_u8, write_u16, write_u32, write_u8, RamRegion};
pub use registry::TargetRegistry;
pub use c_adapter::{CTargetDriver, TargetC, target_list};
