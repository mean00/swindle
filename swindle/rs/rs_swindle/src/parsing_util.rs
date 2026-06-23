//! GDB packet parsing utilities — hex/decimal string conversion.
//!
//! Provides functions for converting between ASCII hex strings and numeric
//! values (u8, u32), as used in the GDB remote protocol. All GDB addresses,
//! data, and register values are transmitted as hex-encoded ASCII strings.

//use alloc::vec::Vec;

setup_log!(false);
//use crate::bmplog;

/// Strip leading whitespace from a string.
pub fn chomp(input: &str) -> &str {
    input.trim_start()
}
/// Convert a hex ASCII string to a u32.
///
/// e.g. `"1234"` => `0x1234`
pub fn ascii_hex_to_u32(sin: &str) -> u32 {
    let datain = sin.as_bytes();
    let mut out: u32 = 0;
    for &i in datain {
        out = (out << 4) + ascii_octet_to_hex(0, i) as u32;
    }
    out
}
/// Convert a hex ASCII byte buffer to a byte array.
///
/// e.g. `"00AABB"` => `[0x00, 0xAA, 0xBB]`
pub fn u8_hex_string_to_u8s<'a>(sin: &'a [u8], sout: &'a mut [u8]) -> &'a [u8] {
    let s = sin.len() / 2;
    for i in 0..s {
        sout[i] = ascii_octet_to_hex(sin[i * 2], sin[i * 2 + 1]);
    }
    &sout[..s]
}

/// Convert an ASCII hex digit to its numeric value.
///
/// e.g. `b'A'` => `10`, `b'3'` => `3`
fn _hex(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit + 10 - b'a',
        b'A'..=b'F' => digit + 10 - b'A',
        _ => 0, // WTF ?
    }
}
/// Convert two ASCII hex digits to a single byte.
///
/// e.g. `(b'F', b'8')` => `0xF8`
pub fn ascii_octet_to_hex(left: u8, right: u8) -> u8 {
    (_hex(left) << 4) + _hex(right)
}
/// Convert a nibble (0-15) to its ASCII hex character.
///
/// e.g. `10` => `b'A'`
pub fn _tohex(v: u8) -> u8 {
    if v >= 10 {
        return b'A' + v - 10;
    }
    b'0' + v
}
/// Convert a decimal or hex string to u32.
///
/// Supports both `"123"` (decimal) and `"0x1234"` (hex) formats.
pub fn ascii_hex_or_dec_to_u32(input: &str) -> u32 {
    if input.starts_with("0x") || input.starts_with("0X") {
        ascii_hex_to_u32(&input[2..])
    } else {
        ascii_string_decimal_to_u32(input)
    }
}
/// Convert a string to a boolean value.
///
/// Accepts `"on"`/`"off"` (case-insensitive) or numeric values (0 = false, non-zero = true).
pub fn string_to_bool(input: &str) -> bool {
    if input == "on" || input == "ON" || input == "On" {
        return true;
    }
    if input == "off" || input == "OFF" || input == "Off" {
        return false;
    }
    ascii_hex_or_dec_to_u32(input) != 0
}
/// Convert a hex ASCII string to u32 (little-endian byte order).
pub fn ascii_string_hex_to_u32_le(s: &str) -> u32 {
    let datain = s.as_bytes();
    u8s_string_to_u32_le(datain)
}

#[allow(dead_code)]
/// Convert two hex ASCII bytes to a u8.
pub fn u8s_string_to_u8(datain: &[u8]) -> u8 {
    if datain.len() < 2 {
        return 0;
    }
    (_hex(datain[0]) << 4) + _hex(datain[1])
}

/// Convert a hex ASCII byte buffer to u32 (little-endian byte order).
pub fn u8s_string_to_u32_le(datain: &[u8]) -> u32 {
    let mut val: u32 = 0;
    let mut shift = 0;
    let number = datain.len() >> 1;

    for i in 0..number {
        let hi: u32 = _hex(datain[i * 2]) as u32;
        let lo: u32 = _hex(datain[i * 2 + 1]) as u32;
        val += ((hi << 4) + lo) << shift;
        shift += 8;
    }
    val
}
/// Convert a decimal ASCII string to u32.
pub fn ascii_string_decimal_to_u32(s: &str) -> u32 {
    u8s_string_decimal_to_u32(s.as_bytes())
}
/// Convert a decimal ASCII byte buffer to u32.
pub fn u8s_string_decimal_to_u32(datain: &[u8]) -> u32 {
    let mut val: u32 = 0;
    for i in datain {
        val = val * 10 + _hex(*i) as u32;
    }
    val
}

/// Convert a hex ASCII string to u32 (big-endian byte order).
pub fn ascii_string_hex_to_u32(s: &str) -> u32 {
    let datain = s.as_bytes();
    u8s_string_to_u32(datain)
}
/// Convert a hex ASCII byte buffer to u32 (big-endian byte order).
pub fn u8s_string_to_u32(datain: &[u8]) -> u32 {
    let mut val: u32 = 0;
    for i in datain {
        val = (val << 4) + _hex(*i) as u32;
    }
    val
}
/// Convert a u32 to a 4-byte little-endian hex ASCII buffer.
pub fn _u32_to_ascii_le_buffer(value: u32, out: &mut [u8]) {
    let mut value = value;
    for i in 0..4 {
        let ascii = u8_to_ascii((value & 0xff) as u8);
        value >>= 8;
        out[i * 2] = ascii[0];
        out[1 + i * 2] = ascii[1];
    }
}
/// Convert a u8 to a 2-character hex ASCII array.
pub fn u8_to_ascii(value: u8) -> [u8; 2] {
    let mut out: [u8; 2] = [0, 0];
    out[0] = _tohex(value >> 4);
    out[1] = _tohex(value & 0xf);
    out
}
/// Write a u8 as 2 hex ASCII characters into a buffer.
pub fn u8_to_ascii_to_buffer(value: u8, out: &mut [u8]) {
    out[0] = _tohex(value >> 4);
    out[1] = _tohex(value & 0xf);
}
/// Parse an "address,length" pair from a GDB command string.
///
/// e.g. `"20001000,100"` => `Some((0x20001000, 0x100))`
pub fn _take_adress_length(xin: &str) -> Option<(u32, u32)> {
    let (addr_str, len_str) = xin.split_once(',')?;
    let address = ascii_string_hex_to_u32(addr_str);
    let len = ascii_string_hex_to_u32(len_str);
    Some((address, len))
}
/// Split a command string at the first `:` separator.
///
/// Returns `(command, arguments)` where arguments may be empty.
pub fn split_command(incoming: &[u8]) -> Option<(&[u8], &[u8])> {
    if let Some(pos) = incoming.iter().position(|&b| b == b':') {
        Some((&incoming[..pos], &incoming[pos + 1..]))
    } else {
        Some((incoming, &[]))
    }
}
//
