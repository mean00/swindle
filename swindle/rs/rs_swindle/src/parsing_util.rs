use alloc::alloc::alloc;
use alloc::alloc::Layout;
use alloc::vec::Vec;
use cty::size_t;

crate::setup_log!(false);
use crate::{bmplog, bmpwarning};
//
//
//
pub fn ascii_hex_string_to_u8s<'a>(sin: &'a str, sout: &'a mut [u8]) -> Result<&'a [u8], i32> {
    let datain = sin.as_bytes();

    let s = datain.len() / 2;
    for i in 0..s {
        sout[i] = ascii_octet_to_hex(datain[i * 2], datain[i * 2 + 1]);
    }
    Ok(&sout[..s])
}
/*
 *
 */
pub fn ascii_hex_string_to_str<'a>(sin: &'a str, sout: &'a mut [u8]) -> Result<&'a str, i32> {
    let datain = sin.as_bytes();

    let s = datain.len() / 2;
    for i in 0..s {
        sout[i] = ascii_octet_to_hex(datain[i * 2], datain[i * 2 + 1]);
    }
    unsafe {
        return Ok(core::str::from_utf8_unchecked(&sout[..s]));
    }
}
//
//
pub fn u8_hex_string_to_u8s<'a>(sin: &'a [u8], sout: &'a mut [u8]) -> &'a [u8] {
    let s = sin.len() / 2;
    for i in 0..s {
        sout[i] = ascii_octet_to_hex(sin[i * 2], sin[i * 2 + 1]);
    }
    &sout[..s]
}

/*
 *
 */
fn _hex(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit + 10 - b'a',
        b'A'..=b'F' => digit + 10 - b'A',
        _ => 0, // WTF ?
    }
}
//---
pub fn ascii_octet_to_hex(left: u8, right: u8) -> u8 {
    (_hex(left) << 4) + _hex(right)
}
//---
pub fn _tohex(v: u8) -> u8 {
    if v >= 10 {
        return b'A' + v - 10;
    }
    b'0' + v
}
//---
pub fn ascii_string_to_u32_le(s: &str) -> u32 {
    let datain = s.as_bytes();
    u8s_string_to_u32_le(datain)
}

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
// decimal!
pub fn ascii_string_decimal_to_u32(s: &str) -> u32 {
    u8s_string_decimal_to_u32(s.as_bytes())
}
pub fn u8s_string_decimal_to_u32(datain: &[u8]) -> u32 {
    let mut val: u32 = 0;
    for i in datain {
        val = val * 10 + _hex(*i) as u32;
    }
    val
}

// Hex string as ascii to u32
pub fn ascii_string_to_u32(s: &str) -> u32 {
    let datain = s.as_bytes();
    u8s_string_to_u32(datain)
}
pub fn u8s_string_to_u32(datain: &[u8]) -> u32 {
    let mut val: u32 = 0;
    for i in datain {
        val = (val << 4) + _hex(*i) as u32;
    }
    val
}

pub fn u32_to_ascii_le_buffer(value: u32, out: &mut [u8]) {
    let mut value = value;
    for i in 0..4 {
        let ascii = crate::parsing_util::u8_to_ascii((value & 0xff) as u8);
        value >>= 8;
        out[0 + i * 2] = ascii[0];
        out[1 + i * 2] = ascii[1];
    }
}

pub fn u8_to_ascii(value: u8) -> [u8; 2] {
    let mut out: [u8; 2] = [0, 0];
    out[0] = _tohex(value >> 4);
    out[1] = _tohex(value & 0xf);
    out
}
pub fn u8_to_ascii_to_buffer(value: u8, out: &mut [u8]) {
    out[0] = _tohex(value >> 4);
    out[1] = _tohex(value & 0xf);
}

pub fn take_adress_length(xin: &str) -> Option<(u32, u32)> {
    let args: Vec<&str> = xin.split(',').collect();
    if args.len() != 2 {
        bmplog!("take_adress_length : wrong param");
        return None;
    }
    let address = crate::parsing_util::ascii_string_to_u32(args[0]);
    let len = crate::parsing_util::ascii_string_to_u32(args[1]);
    Some((address, len))
}

pub fn split_command(incoming: &[u8]) -> Option<(&[u8], &[u8])> {
    let size = incoming.len();
    // look up for the first ':' if any
    // and split there
    for i in 0..size {
        if incoming[i] == b':' {
            // split here
            if i == size - 1 {
                return Some((&incoming[..i], &incoming[0..0]));
            }
            return Some((&incoming[..i], &incoming[(i + 1)..]));
        }
    }
    Some((incoming, &incoming[0..0]))
}
