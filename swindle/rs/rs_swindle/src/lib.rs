//! Swindle GDB stub — a Rust-based debug probe firmware.
//!
//! This crate implements the GDB remote protocol for the Swindle debug probe.
//! It handles GDB commands (read/write registers, memory, flash, breakpoints,
//! watchpoints, stepping, etc.) and dispatches them to the underlying BMP
//! (Black Magic Probe) C library or to the RPC layer for remote probe access.
//!
//! ## Build targets
//!
//! - **Native (embedded)**: Runs directly on the probe MCU (RP2040, GD32, CH32).
//!   Uses `no_std` and accesses hardware via the BMP C library.
//! - **Hosted (desktop)**: Runs on a PC, communicating with a remote probe
//!   over TCP/IP via the RPC protocol.
//!
//! ## Module structure
//!
//! | Module | Description |
//! |--------|-------------|
//! | `bmp` | BMP C FFI bindings |
//! | `bmplogger` | Logging and GDB console output |
//! | `commands` | GDB command handlers (breakpoints, flash, memory, etc.) |
//! | `crc` | CRC32 computation |
//! | `decoder` | GDB packet decoder |
//! | `encoder` | GDB packet encoder |
//! | `freertos` | FreeRTOS task list inspection |
//! | `glue` | C/Rust glue code |
//! | `native` | Native (embedded) build modules |
//! | `hosted` | Hosted (desktop) build modules |
//! | `packet_symbols` | GDB protocol constants |
//! | `parsing_util` | Hex string parsing utilities |
//! | `rpc_common` | Shared RPC protocol definitions |
//! | `rtt` | SEGGER RTT support |
//! | `settings` | Persistent key-value settings |
//! | `sw_breakpoints` | Software breakpoint management |
//! | `swindle` | Main GDB stub loop |
//! | `util` | Memory allocation utilities |

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), no_main)]
#![allow(static_mut_refs)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
//
#[macro_use]
extern crate alloc;
#[cfg(feature = "hosted")]
extern crate std;
//#![allow(unused_imports)]
mod bmp;
#[macro_use]
mod bmplogger;
mod commands;
mod crc;
mod decoder;
mod encoder;
mod freertos;
mod glue;
mod packet_symbols;
mod parsing_util;
mod rn_bmp_cmd_c;
mod rtt_consts;
mod setting_keys;
mod settings;
//mod sync_cell;
//mod rpc;
pub mod rpc_common;
pub mod rpc_common_generated;
// Feature-gated modules: native vs hosted
#[cfg(not(feature = "hosted"))]
pub mod native;
#[cfg(feature = "hosted")]
pub mod hosted;
// RPC modules: hosted vs target
cfg_if::cfg_if! {
    if #[cfg(feature = "hosted")] {
        pub use hosted::{rpc_host, rpc_host_generated};
    } else {
        pub use native::{rpc_target, rpc_target_generated, rpc_target_impl};
    }
}
// rtt and sw_breakpoints are at crate root (contents are #[cfg] guarded)
mod rtt;
mod sw_breakpoints;
// USB vs non-USB transport
cfg_if::cfg_if! {
    if #[cfg(not(any(feature = "hosted", feature = "network")))] {
        mod usb;
    } else {
        mod not_usb;
    }
}
mod util;
//
mod swindle;
pub(crate) use swindle::{rngdb_output_flush, rngdb_send_data, rngdb_send_data_u8};
//
crate::gdb_print_init!();
//
// EOF