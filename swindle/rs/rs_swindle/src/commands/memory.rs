//! GDB memory read/write commands (`m`, `X`).
//!
//! Implements the GDB remote protocol memory access commands:
//!
//! - `maddr,length` — read memory, return hex-encoded bytes
//! - `Xaddr,length:data` — write binary data to memory


use crate::bmp;
use crate::encoder::encoder;
use crate::parsing_util::ascii_string_hex_to_u32;

use crate::bmp::bmp_mem_write;

setup_log!(false);
//use crate::bmplog;

/// Handle `maddr,length` — read memory and return hex-encoded bytes.
pub fn _m(_command: &str, args: &[&str]) -> bool {
    if !bmp::bmp_attached() {
        encoder::reply_e01();
        return true;
    }
    let mut current_address: u32 = ascii_string_hex_to_u32(args[0]);
    let mut left: usize = ascii_string_hex_to_u32(args[1]) as usize;
    let mut tmp: [u8; 16] = [0; 16];
    let mut char_buffer: [u8; 32] = [0; 32];

    let mut e = encoder::new();
    e.begin();

    while left != 0 {
        let chunk: usize = core::cmp::min(16, left);
        bmp::bmp_read_mem(current_address, &mut tmp[0..chunk]);
        left -= chunk;
        for i in 0..chunk {
            crate::parsing_util::u8_to_ascii_to_buffer(tmp[i], &mut char_buffer[(2 * i)..]);
        }
        e.add_u8(&char_buffer[..(2 * chunk)]);
        // avoid overflow
        if left != 0 {
            current_address += chunk as u32;
        }
    }

    e.end();
    true
}
/*
 *
 *
 */
/// Handle `Xaddr,length:data` — write binary data to memory.
pub fn _X(command: &[u8]) -> bool {
    let coma = command.iter().position(|&x| x == b',').unwrap_or(0);
    if coma == 0 {
        return false;
    }
    
    let semicolumn_offset = command[coma..].iter().position(|&x| x == b':').unwrap_or(0);
    if semicolumn_offset == 0 {
        return false;
    }
    let semicolumn = coma + semicolumn_offset;

    let addr_str = unsafe { core::str::from_utf8_unchecked(&command[1..coma]) };
    let len_str = unsafe { core::str::from_utf8_unchecked(&command[(coma + 1)..semicolumn]) };

    let address = ascii_string_hex_to_u32(addr_str);
    let mut length = ascii_string_hex_to_u32(len_str) as usize;

    let data = &command[(semicolumn + 1)..];
    bmplog!("buffer size :  {} bytes\n", data.len());

    bmplog!("Adress : 0x{:x} Len: {}\n", address, length);
    if length > data.len() {
        length = data.len()
    }
    bmplog!("Adress : 0x{:x} Len: {}\n", address, length);
    if length == 0 {
        encoder::reply_ok();
        return true;
    }

    if bmp_mem_write(address, data) {
        encoder::reply_ok();
    } else {
        encoder::reply_e01();
    }
    true
}

// EOF
