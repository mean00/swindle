//! GDB register read/write commands (`g`, `p`, `P`).
//!
//! Implements the GDB remote protocol register access commands:
//!
//! - `g` — read all registers (hex-encoded, little-endian)
//! - `pN` — read a single register N
//! - `PN=v` — write value v to register N

use crate::encoder::encoder;
use crate::parsing_util::{ascii_string_hex_to_u32, ascii_string_hex_to_u32_le};

use crate::bmp;

setup_log!(false);

/// Handle `PN=v` — write a single register.
pub fn _P(_command: &str, args: &[&str]) -> bool {
    let reg: u32 = ascii_string_hex_to_u32(args[0]);
    let val: u32 = ascii_string_hex_to_u32_le(args[1]);
    encoder::reply_bool(bmp::bmp_write_register(reg, val));
    true
}
/// Handle `pN` — read a single register.
pub fn _p(_command: &str, args: &[&str]) -> bool {
    let reg: u32 = ascii_string_hex_to_u32(args[0]);
    match bmp::bmp_read_register(reg) {
        Some(x) => encoder::simple_send_u32_le(x),
        _ => encoder::reply_e01(),
    }
    true
}

/// Handle `g` — read all registers.
pub fn _g(_command: &str, _args: &[&str]) -> bool {
    _g2()
}
/// Read all registers (separate function for easier GDB debugging).
pub extern "C" fn _g2() -> bool {
    let regs = crate::bmp::bmp_read_registers();
    let mut e = encoder::new();

    if regs.is_empty() {
        encoder::simple_send("00000000");
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
