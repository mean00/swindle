//! GDB remote protocol constants — packet framing characters.
//!
//! Defines the control characters used in the GDB remote serial protocol
//! and the internal RPC protocol.

/// Reset character (Ctrl-D).
pub const CHAR_RESET_04: u8 = 0x04;
/// GDB packet start marker (`$`).
pub const CHAR_START: u8 = b'$';
/// GDB packet end marker (`#`), followed by 2-hex-digit checksum.
pub const CHAR_END: u8 = b'#';
/// GDB escape character (`}`) — the next byte is XOR'd with 0x20.
pub const CHAR_ESCAPE: u8 = b'}';
/// Secondary escape character (`{`) — also escaped in output.
pub const CHAR_ESCAPE2: u8 = b'{';

/// RPC session start marker (`+`).
pub const RPC_START_SESSION: u8 = b'+';
/// RPC packet start marker (`!`).
pub const RPC_START: u8 = b'!';
/// RPC packet end marker (`#`).
pub const RPC_END: u8 = b'#';

/// GDB acknowledge character (`+`).
pub const CHAR_ACK: u8 = b'+';
/// GDB negative-acknowledge character (`-`).
pub const CHAR_NACK: u8 = b'-';

/// Maximum GDB input packet size (2112 bytes).
///
/// Must be large enough to hold the largest GDB packet (e.g. memory write
/// with binary data).
pub const INPUT_BUFFER_SIZE: usize = 1024 * 2 + 64;
