//! GDB console output macros.
//!
//! Provides macros for sending text output to the GDB console via `O`
//! packets. The output is hex-encoded and sent as GDB remote protocol
//! `O` packets.
//!
//! ## Macros
//!
//! - `gdb_print!(...)`: Print to GDB console (like `print!`)
//! - `gdb_println!(...)`: Print to GDB console with newline (like `println!`)
//!
//! ## Usage
//!
//! ```ignore
//! gdb_print!("Hello, ");
//! gdb_println!("world!");
//! gdb_println!("Value: ", Hex(0x1234));
//! ```

// Macros are defined in the crate root (lib.rs) and re-exported here.
// This file exists to document the macro functionality.

/// Print formatted output to the GDB console.
///
/// The output is hex-encoded and sent as a GDB `O` packet.
/// Supports `Hex()` formatting for hex values.
#[macro_export]
macro_rules! gdb_print {
    ($($arg:tt)*) => {
        // Implemented in lib.rs
    };
}

/// Print formatted output to the GDB console with a newline.
///
/// The output is hex-encoded and sent as a GDB `O` packet.
/// Supports `Hex()` formatting for hex values.
#[macro_export]
macro_rules! gdb_println {
    ($($arg:tt)*) => {
        // Implemented in lib.rs
    };
}