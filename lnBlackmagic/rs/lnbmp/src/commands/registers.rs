use alloc::vec;
use alloc::vec::Vec;

use super::{exec_one, CommandTree};
use crate::encoder::encoder;
use crate::packet_symbols::INPUT_BUFFER_SIZE;

use super::mon::_swdp_scan;
use crate::bmp::bmp_attached;
use crate::bmp::bmp_get_mapping;
use crate::bmp::mapping::{Flash, Ram};
use crate::bmp::MemoryBlock;

use crate::parsing_util::{ascii_string_to_u32, ascii_string_to_u32_le};

use crate::bmp;

crate::setup_log!(false);
use crate::{bmplog, bmpwarning};

// write register
// Pf=40000008
//

fn pp_prefix(command: &str) -> Option<(u32, u32)> {
    let args: Vec<&str> = command.split('=').collect();
    if args.len() != 2 {
        bmplog!("Pxxx wrong args");
        return None;
    }
    let reg = ascii_string_to_u32(args[0]); // should be small enough for LE/BE to not matter
    let value = ascii_string_to_u32_le(args[1]);
    Some((reg, value))
}
// Write reg
pub fn _P(command: &str, _args: &[&str]) -> bool {
    let reg: u32;
    let val: u32;
    match pp_prefix(&command[1..]) {
        None => {
            encoder::simple_send("E01");
            return true;
        }
        Some((x, y)) => {
            reg = x;
            val = y;
        }
    }
    encoder::reply_bool(bmp::bmp_write_register(reg, val));
    true
}
// read 1 register
pub fn _p(command: &str, _args: &[&str]) -> bool {
    let reg: u32 = crate::parsing_util::ascii_string_to_u32(&command[1..]);
    match bmp::bmp_read_register(reg) {
        Some(x) => encoder::simple_send_u32_le(x),
        _ => encoder::reply_e01(),
    }
    true
}

// Read registers
pub fn _g(_command: &str, _args: &[&str]) -> bool {
    let regs = crate::bmp::bmp_read_registers();
    let mut e = encoder::new();

    if regs.is_empty() {
        encoder::simple_send("0000");
        return true;
    }
    e.begin();
    for i in regs {
        e.add_u32_le(i);
    }
    // now read CSRs
    e.end();
    true
}
