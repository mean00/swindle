// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use crate::encoder::encoder;
use crate::parsing_util::ascii_string_hex_to_u32;

use crate::bmp::bmp_mem_write;

crate::setup_log!(false);
use crate::bmplog;

// memory read m80070f6,4 // m2000000,4

pub fn _m(_command: &str, args: &[&str]) -> bool {
    if !crate::bmp::bmp_attached() {
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
        crate::bmp::bmp_read_mem(current_address, &mut tmp[0..chunk]);
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
/**
 *  \fn write memory
 *  Xaddr,length:XX(binary)
 */
pub fn lookupChar(searched: &str, incoming: &str, initial: usize) -> (bool, usize) {
    match &incoming[initial..].find(searched) {
        Some(x) => (true, x + initial),
        None => (false, 0),
    }
}
/*
 *
 *
 */
pub fn _X(command: &str, _args: &[u8]) -> bool {
    let mut ret: bool;
    let coma: usize;
    let semicolumn: usize;

    (ret, coma) = lookupChar(",", command, 1);
    if !ret {
        return false;
    }
    (ret, semicolumn) = lookupChar(":", command, coma);
    if !ret {
        return false;
    }
    let address = crate::parsing_util::ascii_string_hex_to_u32(&command[1..coma]);
    let mut length =
        crate::parsing_util::ascii_string_hex_to_u32(&command[coma + 1..semicolumn]) as usize;

    let as_byte = command.as_bytes();
    let data = &as_byte[(semicolumn + 1)..];
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
