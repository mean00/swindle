/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the `mon` command tree of blackmagic/src/target/stm32f1.c, made for
 * the Swindle firmware. This file is derived from that C source and stays under the
 * same license as the original Black Magic code reproduced below.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

//! `mon` (GDB `monitor`) commands of the STM32F1-style driver.
//!
//! They are declared through `RustArmTargetDriver::commands()` / `command_help()`, so
//! they only exist while a target driven by this crate is attached (`mon <command>`,
//! listed by `mon help`) — same names and semantics as the former C driver:
//!
//! - `mon option` / `mon option erase` / `mon option <addr> <value>` — dump, unprotect
//!   and program the option bytes
//! - `mon uid` — dump the factory unique device id

use crate::families::f1;
use crate::flash;
use rs_stm32_common::output::print_uid;
use crate::registers::*;
use crate::stm32_parts::PART_GD32_MEDIUM;
use rsbmp::commands::{CallbackType, CommandTree};
use rsbmp::parsing_util::ascii_hex_or_dec_to_u32;
use rsbmp::encoder::encoder;
use rsbmp::format_util::HexCase;
use rsbmp::gdb_println;
use rsbmp::target::{CortexMAccess, get_cur_target};
use rsbmp::bmplogger::G;

/// Commands this driver adds to the generic `mon` tree.
pub const COMMANDS: [CommandTree; 2] = [
    CommandTree {
        command: "option",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_option),
        start_separator: b' ',
        next_separator: b' ',
        help: "Manipulate option bytes",
    },
    CommandTree {
        command: "uid",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_uid),
        start_separator: b' ',
        next_separator: b' ',
        help: "Print unique device ID",
    },
];

/// Handle `mon option` — dump, erase or program the option bytes.
pub fn _option(_command: &str, args: &[&str]) -> bool {
    let Some(info) = f1::active_info() else {
        return false;
    };
    let mut cm = CortexMAccess::new(get_cur_target());

    let read_protected = cm.read32(flash_reg!(obr)) & FPEC_OPTION_BYTE_RDPRT != 0;
    let erase_requested = args.len() == 1 && args[0] == "erase";
    // Fast-exit if the flash is not readable and the user did not ask to unprotect it.
    if read_protected && !erase_requested {
        gdb_println!("Device is Read Protected");
        gdb_println!("Use `monitor option erase` to unprotect and erase device");
        encoder::reply_ok();
        return true;
    }

    if !flash::unlock(&mut cm, FPEC_BANK1_OFFSET) {
        return false;
    }
    cm.write32(flash_reg!(optkeyr), FPEC_KEY1);
    cm.write32(flash_reg!(optkeyr), FPEC_KEY2);

    // GD32E230 is the one part whose 16-bit write does not latch.
    let write16_broken = info.part_id == PART_GD32_MEDIUM && info.cpuid_partno == CORTEX_M23;

    if erase_requested {
        if !flash::option_erase(&mut cm) {
            return false;
        }
        if !flash::option_write_erased(
            &mut cm,
            0,
            flash::readable_key(info.part_id),
            write16_broken,
        ) {
            return false;
        }
    } else if args.len() == 2 {
        let addr = ascii_hex_or_dec_to_u32(args[0]);
        let value = ascii_hex_or_dec_to_u32(args[1]) as u16;
        if !flash::option_write(&mut cm, info.part_id, addr, value) {
            return false;
        }
    } else {
        gdb_println!("usage: monitor option erase");
        gdb_println!("usage: monitor option <addr> <value>");
    }

    let mut values = [0u16; STM32F1_OB_COUNT as usize];
    flash::read_option_bytes(&mut cm, &mut values);
    for (index, value) in values.iter().enumerate() {
        let addr = FPEC_OBP_RDP + index as u32 * 2;
        // C `stm32f1_cmd_option` prints both fields with `%08" PRIX32` / `%04" PRIX32`.
        G::print_hex_padded(addr, ADDR_HEX_DIGITS, HexCase::Upper);
        G::print_str(": ");
        G::print_hex_padded(u32::from(*value), HALFWORD_HEX_DIGITS, HexCase::Upper);
        G::print_str("\n");
    }
    encoder::reply_ok();
    true
}

/// Handle `mon uid` — print the factory unique device id.
pub fn _uid(_command: &str, _args: &[&str]) -> bool {
    let Some(info) = f1::active_info() else {
        return false;
    };
    let uid_base = if flash::readable_key(info.part_id) == FPEC_OBP_RDP_KEY_F3 {
        F3_UID_BASE
    } else {
        F1_UID_BASE
    };
    let cm = CortexMAccess::new(get_cur_target());
    if !print_uid(&cm, uid_base) {
        return false;
    }
    encoder::reply_ok();
    true
}
