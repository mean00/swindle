// hosted/mod.rs — Re-exports for hosted (PC) mode
// Included when #[cfg(feature = "hosted")]

pub mod rpc_host;
pub mod rpc_host_generated;

#[macro_use]
pub mod logger_hosted;