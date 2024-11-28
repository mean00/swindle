use crate::bmp;
use crate::commands::{exec_one, mon, CallbackType, CommandTree, HelpTree};
use crate::encoder::encoder;
use crate::freertos::enable_freertos;
use crate::parsing_util::{
    ascii_hex_string_to_u8s, ascii_hex_to_u32, ascii_string_decimal_to_u32, ascii_string_hex_to_u32,
};
use alloc::vec;
use alloc::vec::Vec;
//
//
crate::setup_log!(false);
crate::gdb_print_init!();
use crate::{bmplog, bmpwarning, gdb_print};
///
pub const ch32vxx_command_tree: [CommandTree; 2] = [
    CommandTree {
        command: "ch32v3_obr",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_ch32v3_obr),
        start_separator: " ",
        next_separator: " ",
    }, //
    CommandTree {
        command: "ch32v3_option_byte",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_ch32v3_option_byte),
        start_separator: " ",
        next_separator: " ",
    }, //
];
//
pub const ch32vxx_help_tree : [HelpTree;2]=
[
    HelpTree{ command: "ch32v3_obr",help :"Read/write the read protection status."},
    HelpTree{ command: "ch32v3_option_byte",help :"Read/write the user option byte on ch32v3 chip.\n\tThat changes the flash/ram split.\n\tUsual value : 256/64 -> 0x9f, 192/128 -> 0x1f."},
];

/*
 *
 */
const CH32V3XX_USER_OPTION_ADDR: u32 = 0x1ffff800;
const CH32V3XX_FLASH_OBR_ADR: u32 = 0x4002201C;
const CH32V3XX_OBR_ERROR: u32 = 1 << 0;
const CH32V3XX_OBR_RDP_VALID: u32 = 1 << 1;
//
//
pub fn _ch32v3_obr(_command: &str, args: &[&str]) -> bool {
    let mut value: [u32; 1] = [0];
    if args.is_empty() {
        if !bmp::bmp_read_mem32(CH32V3XX_FLASH_OBR_ADR, &mut value) {
            gdb_print!("Error reading OBR register\n");
            return false;
        }
        gdb_print!("OBR : {:x}\n", value[0]);
        if (value[0] & CH32V3XX_OBR_ERROR) != 0 {
            gdb_print!("OBR invalid configuration , inverted option does not match not inverted\n");
        } else {
            gdb_print!("OBR configuration is valid\n");
        }

        if (value[0] & CH32V3XX_OBR_RDP_VALID) != 0 {
            gdb_print!("OBR Read protection is valid \n");
        } else {
            gdb_print!("OBR Read protection is not valid \n");
        }
        encoder::reply_ok();
        return true;
    }
    // it is a write, get the value
    //let value = detail[1];
    //let value_u8: u8 = if value.starts_with("0x") || value.starts_with("0X") {
    //(ascii_hex_to_u32(&value[2..]) & 0xff) as u8
    //} else {
    //(ascii_string_decimal_to_u32(value) & 0xff) as u8
    //};
    //if (value_u8 & 0x7) != 0x7 {
    //gdb_print!("The provided value does not seem valid \n");
    //return false;
    //}
    //bmp::bmp_ch32v3xx_write_user_option_byte(value_u8);
    encoder::reply_ok();
    true
}

pub fn _ch32v3_option_byte(_command: &str, args: &[&str]) -> bool {
    let mut value: [u32; 1] = [0];
    if args.is_empty() {
        // now read option
        if !bmp::bmp_read_mem32(CH32V3XX_USER_OPTION_ADDR, &mut value) {
            gdb_print!("Error reading user option\n");
            return false;
        }
        let mut option = (value[0] >> 16) & 0xff;
        gdb_print!("option byte 0x{:x}\n", option);
        if (option & 0x7) != 0x7 {
            gdb_print!("invalid memory configuration\n");
            return false;
        }
        option >>= 5;
        let flash: usize;
        let ram: usize;
        match option {
            0 | 1 => (flash, ram) = (192, 128),
            2 | 3 => (flash, ram) = (224, 96),
            4 | 5 => (flash, ram) = (256, 64),
            6 => (flash, ram) = (128, 192),
            7 => (flash, ram) = (288, 32),
            _ => {
                (flash, ram) = (224, 96);
                gdb_print!("invalid memory configuration\n");
            }
        }
        gdb_print!("Flash {} kB, Ram {} kB\n", flash, ram);
        encoder::reply_ok();
        return true;
    }
    // it is a write, get the value
    let value = args[0];
    let value_u8: u8 = if value.starts_with("0x") || value.starts_with("0X") {
        (ascii_hex_to_u32(&value[2..]) & 0xff) as u8
    } else {
        (ascii_string_decimal_to_u32(value) & 0xff) as u8
    };
    if (value_u8 & 0x7) != 0x7 {
        gdb_print!("The provided value does not seem valid \n");
        return false;
    }
    bmp::bmp_ch32v3xx_write_user_option_byte(value_u8);
    encoder::reply_ok();
    true
}
