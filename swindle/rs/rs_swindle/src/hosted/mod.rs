// hosted/mod.rs — Re-exports for hosted (PC) mode
// Included when #[cfg(feature = "hosted")]

pub mod bulk_mem;
pub mod c_stubs;
pub mod dmi_scan;
pub mod rpc_host;
pub mod rpc_host_generated;
pub mod transport;

#[macro_use]
pub mod logger_hosted;