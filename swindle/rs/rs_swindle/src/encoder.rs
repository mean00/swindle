//! GDB packet encoder — builds and sends GDB remote protocol packets.
//!
//! Constructs GDB packets with proper framing (`$...`), checksum, and escape
//! handling. Designed to avoid heap allocation and minimise memory copies by
//! using a small static temporary buffer.
//!
//! ## Usage
//!
//! ```ignore
//! let mut e = encoder::new();
//! e.begin();          // send '$'
//! e.add("command");   // add data
//! e.add_u32(value);   // add hex-encoded u32
//! e.end();            // send '#' + checksum + flush
//! ```
//!
//! ## Pre-built replies
//!
//! - `reply_ok()` — sends `$OK#9A`
//! - `reply_e01()` — sends `$E01#A6`
//! - `reply_bool(bool)` — sends OK or E01

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_imports)]

use crate::packet_symbols;
use crate::parsing_util::{u8_to_ascii, u8_to_ascii_to_buffer};
use crate::{rngdb_output_flush, rngdb_send_data, rngdb_send_data_u8};
use core::ptr::addr_of_mut;

// DF -- not thread safe, not re-entrant,ugly but simple
const TEMP_BUFFER_SIZE: usize = 64;
static mut temp_buffer: [u8; TEMP_BUFFER_SIZE] = [0; TEMP_BUFFER_SIZE];

setup_log!(false);
//use crate::{bmplog, bmpwarning};

/// GDB packet encoder.
///
/// Tracks checksum and escape state while building a packet. Uses a static
/// temporary buffer to avoid heap allocation. Not thread-safe — use only
/// from a single execution context.
pub struct encoder {
    checksum: usize,
    escaped: bool,
}

// this buffer is NOT thread safe and should be used
// only in a single shot

/// Get a reference to the static temporary buffer.
///
/// Not thread-safe — intended for single-shot use within the GDB stub.
fn get_temp_buffer() -> &'static mut [u8] {
    unsafe { &mut *addr_of_mut!(temp_buffer) }
}
//
//
impl encoder {
    /// Create a new encoder with zero checksum and no escape state.
    pub fn new() -> Self {
        encoder {
            checksum: 0,
            escaped: false,
        }
    }
    /// Send `$OK#9A` or `$E01#A6` depending on `status`.
    pub fn reply_bool(status: bool) {
        if status {
            Self::reply_ok();
        } else {
            Self::reply_e01();
        }
    }
    /// Send the GDB OK response (`$OK#9A`).
    pub fn reply_ok() {
        Self::raw_send_u8(b"$OK#9A");
        Self::flush();
    }
    /// Send the GDB error response (`$E01#A6`).
    pub fn reply_e01() {
        Self::raw_send_u8(b"$E01#A6");
        Self::flush();
    }
    /// Add a u32 value as 8 hex digits (big-endian).
    pub fn add_u32(&mut self, val: u32) {
        let mut tmp: [u8; 8] = [0; 8];

        let mut shift = 24;
        let mut offset = 0;
        for _i in 0..4 {
            let digit: u8 = ((val >> shift) & 0xff) as u8;
            u8_to_ascii_to_buffer(digit, &mut tmp[offset..(offset + 2)]);
            shift -= 8;
            offset += 2;
        }
        self.add_u8(&tmp);
    }
    /// Add a u32 value as hex digits, omitting leading zeros.
    ///
    /// If the value is zero, sends `"00"`.
    pub fn add_u32_no_padding(&mut self, val: u32) {
        let mut tmp: [u8; 8] = [0; 8];

        let mut shift = 24;
        let mut offset = 0;
        let mut nb_digit: usize = 0;

        for _i in 0..4 {
            let digit: u8 = ((val >> shift) & 0xff) as u8;
            if digit == 0 && nb_digit == 0 {
            } else {
                u8_to_ascii_to_buffer(digit, &mut tmp[offset..(offset + 2)]);
                nb_digit += 2;
            }
            shift -= 8;
            offset += 2;
        }
        if nb_digit == 0 {
            nb_digit = 2;
            tmp[0] = b'0';
            tmp[1] = b'0';
        }
        self.add_u8(&tmp[(8 - nb_digit)..8]);
    }
    /// Add a u32 value as 8 hex digits (little-endian byte order).
    pub fn add_u32_le(&mut self, val: u32) {
        let mut tmp: [u8; 8] = [0; 8];

        let mut shift = 0;
        let mut offset = 0;
        for _i in 0..4 {
            let digit: u8 = ((val >> shift) & 0xff) as u8;
            u8_to_ascii_to_buffer(digit, &mut tmp[offset..(offset + 2)]);
            shift += 8;
            offset += 2;
        }

        self.add_u8(&tmp);
    }
    /// Convenience: build and send a packet containing a single LE u32.
    pub fn simple_send_u32_le(val: u32) {
        let mut e = Self::new();

        e.begin();
        e.add_u32_le(val);
        e.end();
    }
    /// Start a new GDB packet: send `$` and reset checksum/escape state.
    pub fn begin(&mut self) {
        self.checksum = 0;
        self.escaped = false;
        Self::raw_send_u8(&[packet_symbols::CHAR_START]);
    }
    /// Hex-encode raw bytes and send them directly (no packet framing).
    ///
    /// Used for sending binary data as hex without GDB packet overhead.
    /// Only available in native (non-hosted) builds.
    #[cfg(not(feature = "hosted"))]
    pub fn hexify_and_raw_send(str: &[u8]) {
        let buffer = get_temp_buffer();
        let mut byt = str;
        while !byt.is_empty() {
            let n = core::cmp::min(byt.len(), TEMP_BUFFER_SIZE / 2);
            for i in 0..n {
                u8_to_ascii_to_buffer(byt[i], &mut buffer[2 * i..]);
            }
            Self::raw_send_u8(&buffer[..2 * n]);
            byt = &byt[n..];
        }
    }
    /// Hex-encode a string and add it to the current packet.
    ///
    /// Used for GDB `O` packets (console output) where text must be
    /// hex-encoded.
    pub fn hex_and_add(&mut self, data: &str) {
        let mut byt = data.as_bytes();
        let buffer = get_temp_buffer();
        while !byt.is_empty() {
            let n = core::cmp::min(byt.len(), TEMP_BUFFER_SIZE / 2);
            for i in 0..n {
                u8_to_ascii_to_buffer(byt[i], &mut buffer[2 * i..]);
            }
            self.add_u8(&buffer[..2 * n]);
            byt = &byt[n..];
        }
    }
    /// Add raw bytes to the current packet, escaping special characters.
    ///
    /// Escapes `$`, `#`, `}`, and `*` characters per the GDB remote protocol.
    pub fn add_u8(&mut self, byt: &[u8]) {
        let mut n = 0;
        let buffer = get_temp_buffer();

        for c in byt {
            match *c {
                packet_symbols::CHAR_ESCAPE2
                | packet_symbols::CHAR_ESCAPE
                | packet_symbols::CHAR_START
                | packet_symbols::CHAR_END => {
                    buffer[n] = packet_symbols::CHAR_ESCAPE;
                    buffer[n + 1] = *c ^ 0x20;
                    n += 2;
                    self.checksum += (*c ^ 0x20) as usize;
                    self.checksum += packet_symbols::CHAR_ESCAPE as usize;
                }
                _ => {
                    buffer[n] = *c;
                    n += 1;
                    self.checksum += *c as usize;
                }
            };
            if n > TEMP_BUFFER_SIZE - 2 {
                Self::raw_send_u8(&buffer[0..n]);
                n = 0;
            }
        }
        if n > 0 {
            Self::raw_send_u8(&buffer[0..n]);
        }
    }
    /// Add a string to the current packet (convenience wrapper).
    pub fn add(&mut self, data: &str) {
        self.add_u8(data.as_bytes());
    }
    /// Finalise the packet: send `#`, the 2-hex-digit checksum, and flush.
    pub fn end(&mut self) {
        // send end
        Self::raw_send_u8(&[packet_symbols::CHAR_END]);
        // Send checksum
        Self::raw_send_u8(&u8_to_ascii((self.checksum & 0xff) as u8));
        Self::flush();
    }
    /// Send a GDB error packet `E<code>`.
    #[allow(dead_code)]
    pub fn error(code: u8) {
        let mut e = Self::new();
        let zero: char = (b'0' + code) as char;
        let mut tmp = [0u8; 4];
        e.begin();
        e.add("E");
        e.add(zero.encode_utf8(&mut tmp));
        e.end();
    }
    /// Convenience: build and send a packet containing a single string.
    pub fn simple_send(data: &str) {
        let mut e = Self::new();
        e.begin();
        e.add(data);
        e.end();
    }
    /// Send raw string data without packet framing.
    #[allow(dead_code)]
    pub fn raw_send(data: &str) {
        rngdb_send_data_u8(data.as_bytes());
    }
    /// Send raw byte data without packet framing.
    pub fn raw_send_u8(data: &[u8]) {
        rngdb_send_data_u8(data);
    }
    /// Flush the output buffer to the transport layer.
    pub fn flush() {
        rngdb_output_flush();
    }
}
