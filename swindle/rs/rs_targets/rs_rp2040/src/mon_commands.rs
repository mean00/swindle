//! `mon` (GDB `monitor`) commands of the RP2040 driver.
//!
//! Rust port of `rp_cmd_list` from `blackmagic/src/target/rp2040.c`:
//!
//! - `mon erase_sector [start] length` — erase flash without going through GDB's `load`
//! - `mon reset_usb_boot [gpio_mask] [activity_led]` — reboot into BOOTSEL mode
//!
//! Both work on their own because the command callbacks get no driver handle: they resolve
//! the live driver in the ARM dispatcher ([`crate::driver::active_target()`]) and drive the
//! flash controller through [`crate::flash`] for the duration of the call.

use crate::driver::active_target;
use crate::flash::SsiEngine;
use crate::registers::*;
use rsbmp::commands::{CallbackType, CommandTree};
use rsbmp::encoder::encoder;
use rsbmp::gdb_println;
use rsbmp::parsing_util::ascii_hex_or_dec_to_u32;
use rsbmp::target::{CortexMAccess, get_cur_target};

/// Words of the Cortex-M register file the boot ROM call needs (C `uint32_t regs[20]`).
const REGISTER_COUNT: usize = 20;

/// Commands this driver adds to the generic `mon` tree (C `rp_cmd_list`).
pub const COMMANDS: [CommandTree; 2] = [
    CommandTree {
        command: "erase_sector",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_erase_sector),
        start_separator: b' ',
        next_separator: b' ',
        help: "Erase a sector: [start address] length",
    },
    CommandTree {
        command: "reset_usb_boot",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_reset_usb_boot),
        start_separator: b' ',
        next_separator: b' ',
        help: "Reboot the device into BOOTSEL mode",
    },
];

/// Handle `mon erase_sector` — erase `length` bytes of flash at `start`.
///
/// Without a start address the erase begins at the base of the flash, like the C command
/// whose default is `target->flash->start`.
pub fn _erase_sector(_command: &str, args: &[&str]) -> bool {
    let Some(target) = active_target() else {
        return false;
    };

    // C `rp_cmd_erase_sector()` takes either (start, length) or just (length). The
    // length itself is not used: erasing a block already erases exactly one sector, the
    // same way the C `bmp_spi_flash_erase()` ignores it.
    let (start, _length) = match args {
        [start, length] => (
            ascii_hex_or_dec_to_u32(start),
            ascii_hex_or_dec_to_u32(length),
        ),
        [length] => (target.flash.start, ascii_hex_or_dec_to_u32(length)),
        _ => return false,
    };

    let cm = CortexMAccess::new(get_cur_target());
    // The C command brackets the erase with the flash mode hooks; taking the SSI over for
    // the call is all it takes to hand it back afterwards.
    let Some(mut engine) = SsiEngine::engage(&cm, target.flash, false) else {
        return false;
    };
    let erased = engine.erase(&cm, start);
    engine.release(&cm);

    if !erased {
        return false;
    }
    encoder::reply_ok();
    true
}

/// Handle `mon reset_usb_boot` — jump into the boot ROM's USB boot mode.
pub fn _reset_usb_boot(_command: &str, args: &[&str]) -> bool {
    let Some(target) = active_target() else {
        return false;
    };
    let Some(usb_boot) = target.rom_reset_usb_boot else {
        gdb_println!("No boot ROM function table entry found");
        return false;
    };

    // The boot ROM routine takes its arguments in r0 and r1.
    let mut regs = [0u32; REGISTER_COUNT];
    if let Some(value) = args.first() {
        regs[0] = ascii_hex_or_dec_to_u32(value);
    }
    if let Some(value) = args.get(1) {
        regs[1] = ascii_hex_or_dec_to_u32(value);
    }
    regs[CORTEX_REG_PC] = u32::from(usb_boot);
    // A dummy return address, as if the part had just booted.
    regs[CORTEX_REG_LR] = u32::MAX;
    // Stack at the end of SRAM, and the status register set up for Thumb execution.
    regs[CORTEX_REG_MSP] = SRAM_BASE + SRAM_SIZE as u32;
    regs[CORTEX_REG_XPSR] = CORTEXM_XPSR_THUMB;

    // The register file is a little-endian array of 32-bit words.
    let mut data = [0u8; REGISTER_COUNT * 4];
    for (word, chunk) in regs.iter().zip(data.chunks_exact_mut(4)) {
        chunk.copy_from_slice(&word.to_le_bytes());
    }

    let cm = CortexMAccess::new(get_cur_target());
    if !cm.regs_write(&data) {
        return false;
    }
    // The routine never returns, so just resume the core.
    cm.halt_resume(false);
    encoder::reply_ok();
    true
}
