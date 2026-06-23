//! RTT symbol name constants.
//!
//! The symbol name that the debugger searches for in the target's ELF
//! to locate the SEGGER RTT control block.

/// ELF symbol name for the SEGGER RTT control block.
pub const RTTSymbolName: [&str; 1] = ["_SEGGER_RTT"];
