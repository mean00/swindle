//! GDB `vFlash` commands — flash programming support.
//!
//! Implements the GDB remote protocol flash programming sequence:
//!
//! 1. `vFlashErase:addr,length` — erase a region of flash
//! 2. `vFlashWrite:addr,data` — write data to flash
//! 3. `vFlashDone` — finalise the flash operation
//!
//! The `DISABLE_FLASH` flag can be set to `true` to skip actual flash
//! operations (useful for testing).


use super::{CommandTree, exec_one};
use crate::encoder::encoder;

use crate::bmp::{bmp_flash_complete, bmp_flash_erase, bmp_flash_write};
use crate::commands::CallbackType;
use crate::parsing_util::ascii_string_hex_to_u32;

setup_log!(false);
//use crate::bmplog;

const vflash_command_tree: [CommandTree; 3] = [
    CommandTree {
        command: "vFlashErase", // vFlashErase:00000000,00006200
        min_args: 2,
        require_connected: true,
        cb: CallbackType::text(_vFlashErase),
        start_separator: b':',
        next_separator: b',',
    }, // flash erase
    CommandTree {
        command: "vFlashWrite",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::raw(_vFlashWrite),
        start_separator: 0,
        next_separator: 0,
    }, // flash write
    CommandTree {
        command: "vFlashDone",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_vFlashDone),
        start_separator: 0,
        next_separator: 0,
    }, // flash erase
];

const DISABLE_FLASH: bool = false;

//
//
//

/// Dispatch `vFlash*` commands to the appropriate handler.
pub fn _flashv(command: &[u8]) -> bool {
    exec_one(&vflash_command_tree, command)
}

//
//vFlashErase:08000000,00005000
/// Handle `vFlashErase:addr,length` — erase a flash region.
fn _vFlashErase(_command: &str, args: &[&str]) -> bool {
    if DISABLE_FLASH {
        encoder::reply_ok();
        return true;
    }
    let address = ascii_string_hex_to_u32(args[0]);
    let length = ascii_string_hex_to_u32(args[1]);

    encoder::reply_bool(bmp_flash_erase(address, length));
    true
}
//
//vFlashWrite:08000000,data
/// Handle `vFlashWrite:addr,data` — write data to flash.
///
/// Parses the address and binary data from the raw command string.
fn _vFlashWrite(command: &[u8]) -> bool {
    if DISABLE_FLASH {
        encoder::reply_ok();
        return true;
    }

    let block: &[u8] = &command[12..];
    let len = block.len();

    if len < 4 {
        bmplog!("flashWrite: invalid arg1 \n");
        encoder::reply_e01();
        return true;
    }

    // lookup for the ":"
    let mut prefix: usize = 0;
    let mut index: usize = 0;
    while prefix == 0 && index < 16 {
        if block[index] == b':' {
            prefix = index;
        }
        index += 1;
    }
    if prefix == 0 {
        bmplog!("flashWrite: invalid arg2");
        encoder::reply_e01();
        return true;
    }

    let adr = crate::parsing_util::u8s_string_to_u32(&block[..prefix]);
    let data: &[u8] = &block[(prefix + 1)..];
    bmplog!("adr:0x{:x} en:{}\n", adr, data.len());
    bmplog!("write : Adr 0x{:x} len {}\n", adr, len);

    encoder::reply_bool(bmp_flash_write(adr, data));
    true
}
//vFlashDone
/// Handle `vFlashDone` — finalise the flash operation.
fn _vFlashDone(_command: &str, _args: &[&str]) -> bool {
    if DISABLE_FLASH {
        encoder::reply_ok();
        return true;
    }

    encoder::reply_bool(bmp_flash_complete());
    true
}

// EOF
