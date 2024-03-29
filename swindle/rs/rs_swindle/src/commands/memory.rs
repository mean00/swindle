// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use super::{exec_one, CommandTree};
use crate::encoder::encoder;

use crate::bmp::{bmp_attach, bmp_flash_erase, bmp_mem_write};

crate::setup_log!(false);
use crate::{bmplog, bmpwarning};

// memory read m80070f6,4
pub fn _m(command: &str, _args: &[&str]) -> bool {
    if !crate::bmp::bmp_attached() {
        encoder::reply_e01();
        return true;
    }

    match crate::parsing_util::take_adress_length(&command[1..]) {
        None => encoder::reply_e01(),
        Some((adr, len)) => {
            let mut tmp: [u8; 16] = [0; 16];
            let mut char_buffer: [u8; 32] = [0; 32];

            let mut current_address: u32 = adr;
            let mut left: usize = len as usize;

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
        }
    }
    true
}
/**
 *  \fn write memory
 *
 */
pub fn _X(command: &str, args: &[u8]) -> bool {
    match crate::parsing_util::take_adress_length(&command[1..]) {
        None => encoder::reply_e01(),
        Some((addr, len)) => {
            bmplog!("adr: 0x{:x} en:{}\n", addr, len);
            let mut actual_len: usize = len as usize;
            if args.len() > actual_len {
                actual_len = args.len();
            }
            if bmp_mem_write(addr, &args[0..actual_len]) {
                encoder::reply_ok();
            } else {
                encoder::reply_e01();
            }
        }
    };
    true
}

// EOF
