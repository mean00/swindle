// native/mod.rs — Re-exports for native (embedded) mode
// Included when #[cfg(not(feature = "hosted"))]

pub mod rpc_target;
pub mod rpc_target_generated;
pub mod rpc_target_impl;
pub mod cdc_logger;
pub mod bmp_native;

#[macro_use]
pub mod logger_native;
