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
    // One BMP C read call per chunk: `tmp` holds the raw bytes, `char_buffer`
    // the hex encoding (2 chars/byte); together 64 + 128 = 192 B of stack.
    let mut tmp: [u8; crate::mem_cache::READ_CHUNK_SIZE] = [0; crate::mem_cache::READ_CHUNK_SIZE];
    let mut char_buffer: [u8; crate::mem_cache::READ_CHUNK_SIZE * 2] =
        [0; crate::mem_cache::READ_CHUNK_SIZE * 2];

    let mut e = encoder::new();
    e.begin();

    while left != 0 {
        let chunk: usize = core::cmp::min(crate::mem_cache::READ_CHUNK_SIZE, left);
        if chunk <= crate::mem_cache::LINE_SIZE {
            // Small reads are served from the read-ahead line cache when
            // possible; otherwise the whole 16-byte line is prefetched
            // (RAM/flash only) so subsequent small reads cost zero SWD
            // transactions.
            if !crate::mem_cache::try_read(current_address, chunk, &mut tmp[..chunk]) {
                let base = current_address & crate::mem_cache::ALIGN_MASK;
                let off = (current_address - base) as usize;
                let fits_in_line = off + chunk <= crate::mem_cache::LINE_SIZE;
                if fits_in_line && crate::mem_cache::cacheable(base, crate::mem_cache::LINE_SIZE as u32) {
                    let mut line: [u8; crate::mem_cache::LINE_SIZE] = [0; crate::mem_cache::LINE_SIZE];
                    if bmp::bmp_read_mem(base, &mut line) {
                        crate::mem_cache::fill(base, &line);
                        tmp[..chunk].copy_from_slice(&line[off..off + chunk]);
                    } else {
                        bmp::bmp_read_mem(current_address, &mut tmp[..chunk]);
                    }
                } else {
                    bmp::bmp_read_mem(current_address, &mut tmp[..chunk]);
                }
            }
        } else {
            bmp::bmp_read_mem(current_address, &mut tmp[..chunk]);
        }
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
        // Memory changed: the read-ahead line cache must not serve stale bytes.
        crate::mem_cache::invalidate();
        encoder::reply_ok();
    } else {
        encoder::reply_e01();
    }
    true
}

// EOF
