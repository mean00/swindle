//! `mon` (GDB `monitor`) commands of the CH32V1xx / CH32V2xx / CH32V3xx driver.
//!
//! They are declared by `Ch32v123xxDriver` through `RustRiscvTargetDriver::commands()` /
//! `command_help()`, so they only exist while a target driven by this crate is
//! attached (`mon <command>`, listed by `mon help`):
//!
//! - `mon ch32v3_option_byte [value]` — read/write the user option byte
//!   (controls the flash/RAM split, e.g. 256/64 → 0x9f, 192/128 → 0x1f)
//! - `mon ch32v3_obr [value]` — read/write the read protection (OBR) status

use crate::registers::OPTION_BYTES_ADDRESS;
use rsbmp::bmp;
use rsbmp::commands::{CallbackType, CommandTree};
use rsbmp::encoder::encoder;
use rsbmp::parsing_util::ascii_hex_or_dec_to_u32;
use rsbmp::{gdb_print, gdb_println};

/// `FLASH_OBR` of the CH32V2/V3 flash controller.
const FLASH_OBR_ADDRESS: u32 = 0x4002_201c;
/// `FLASH_WPR` of the CH32V2/V3 flash controller.
const FLASH_WPR_ADDRESS: u32 = 0x4002_2020;
/// `FLASH_OBR` bit 0: the inverted option byte does not match the option byte.
const OBR_ERROR: u32 = 1 << 0;
/// `FLASH_OBR` bit 1: the read protection is active.
const OBR_RDP_VALID: u32 = 1 << 1;

/// Commands this driver adds to the generic `mon` tree.
pub const COMMANDS: [CommandTree; 2] = [
    CommandTree {
        command: "ch32v3_obr",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_ch32v3_obr),
        start_separator: b' ',
        next_separator: b' ',
        help: "Read/write the read protection status.",
    },
    CommandTree {
        command: "ch32v3_option_byte",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_ch32v3_option_byte),
        start_separator: b' ',
        next_separator: b' ',
        help: "Read/write the user option byte on ch32v3 chip.\n\tThat changes the flash/ram split.\n\tUsual value : 256/64 -> 0x9f, 192/128 -> 0x1f.",
    },
];

/// Handle `mon ch32v3_obr` — read/write the read protection status.
pub fn _ch32v3_obr(_command: &str, args: &[&str]) -> bool {
    let mut value: [u32; 1] = [0];
    if !bmp::bmp_read_mem32(FLASH_WPR_ADDRESS, &mut value) {
        gdb_print!("Error reading WPR register\n");
        return false;
    }
    gdb_print!("Write Prot : ", Hex(value[0]));
    gdb_print!("\n");
    if args.is_empty() {
        if !bmp::bmp_read_mem32(FLASH_OBR_ADDRESS, &mut value) {
            gdb_print!("Error reading OBR register\n");
            return false;
        }
        gdb_println!("OBR : ", Hex(value[0]));
        gdb_println!("\t Error      : ", value[0] & (1 << 0));
        gdb_println!("\t ReadProt   : ", (value[0] & (1 << 1)) >> 1);
        gdb_println!("\t WDG enable : ", (value[0] & (1 << 2)) >> 2);
        gdb_println!("\t Stop Reset : ", (value[0] & (1 << 3)) >> 3);
        gdb_println!("\t Stby Reset : ", (value[0] & (1 << 4)) >> 4);
        gdb_println!("\t Sram code  : ", (value[0] >> 8) & 3);
        if (value[0] & OBR_ERROR) != 0 {
            gdb_print!("\nOBR invalid configuration , inverted option does not match not inverted");
        } else {
            gdb_print!("\nOBR configuration is valid");
        }

        if (value[0] & OBR_RDP_VALID) != 0 {
            gdb_print!("\nOBR Read protection is valid ");
        } else {
            gdb_print!("\nOBR Read protection is not valid ");
        }
        gdb_print!("\n");
        encoder::reply_ok();
        return true;
    }
    // it is a write, get the value
    let new_value = ascii_hex_or_dec_to_u32(args[0]);
    if new_value == 0 {
        gdb_print!("The provided value does not seem valid (0x27c is valid)\n");
        return false;
    }
    gdb_print!("Writing OBR is not implemented !\n");
    false
}

/// Handle `mon ch32v3_option_byte` — read/write the user option byte.
pub fn _ch32v3_option_byte(_command: &str, args: &[&str]) -> bool {
    let mut value: [u32; 1] = [0];
    if args.is_empty() {
        // now read option
        if !bmp::bmp_read_mem32(OPTION_BYTES_ADDRESS, &mut value) {
            gdb_print!("Error reading user option\n");
            return false;
        }
        let mut option = (value[0] >> 16) & 0xff;
        gdb_print!("Option byte:", Hex(option));
        gdb_print!("\n");
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
        gdb_print!("Flash (kB):", flash);
        gdb_print!("\nRam (kB):", ram);
        gdb_print!("\n");
        encoder::reply_ok();
        return true;
    }
    // it is a write, get the value
    let value: u32 = ascii_hex_or_dec_to_u32(args[0]);
    if (value & 0x7) != 0x7 {
        gdb_print!("The provided value does not seem valid \n");
        return false;
    }
    bmp::bmp_ch32v3xx_write_user_option_byte((value & 0xFF) as u8);
    encoder::reply_ok();
    true
}
