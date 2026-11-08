/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011 Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2017, 2018 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (C) 2023-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the `mon` command tree of blackmagic/src/target/stm32f4.c, made for
 * the Swindle firmware.
 */

//! `mon` (GDB `monitor`) commands of the STM32F2/F4/F7 and GD32F4 driver.

use crate::driver;
use crate::flash;
use rs_stm32_common::output::{partial_match, print_uid};
use crate::registers::*;
use rsbmp::bmplogger::G;
use rsbmp::commands::{CallbackType, CommandTree};
use rsbmp::encoder::encoder;
use rsbmp::format_util::HexCase;
use rsbmp::gdb_println;
use rsbmp::parsing_util::ascii_hex_or_dec_to_u32;
use rsbmp::target::{CortexMAccess, get_cur_target};

/// Commands this driver adds to the generic `mon` tree.
pub const COMMANDS: [CommandTree; 3] = [
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
        command: "psize",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_psize),
        start_separator: b' ',
        next_separator: b' ',
        help: "Configure flash write parallelism: (x8|x16|x32(default)|x64)",
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

/// Handle `mon option` — dump, erase or program the option words.
pub fn _option(_command: &str, args: &[&str]) -> bool {
    let Some(info) = driver::active_info() else {
        return false;
    };
    let mut cm = CortexMAccess::new(get_cur_target());
    let opt_words = flash::option_word_count(info.part_id);

    if args.len() == 1 && partial_match(args[0], "erase") {
        let (mut val, count) = flash::option_defaults(info.part_id);
        if !flash::option_write(&mut cm, info.part_id, &mut val, count) {
            return false;
        }
    } else if args.len() > 1 && partial_match(args[0], "write") {
        let mut val = [0u32; 3];
        for (index, arg) in args[1..].iter().take(3).enumerate() {
            val[index] = ascii_hex_or_dec_to_u32(arg);
        }
        let count = args.len().min(3);
        if flash::option_mask(info.part_id, &mut val) {
            if !flash::option_write(&mut cm, info.part_id, &mut val, count) {
                return false;
            }
        } else {
            gdb_println!("error");
        }
    } else {
        gdb_println!("usage: monitor option erase");
        G::print_str("usage: monitor option write <OPTCR>");
        if opt_words > 1 {
            G::print_str(" <OPTCR1>");
        }
        if opt_words > 2 {
            G::print_str(" <OPTCR2>");
        }
        G::print_str("\n");
    }

    let mut val = [0u32; 3];
    flash::read_options(&cm, info.part_id, &mut val);
    flash::option_mask(info.part_id, &mut val);
    G::print_str("OPTCR: ");
    G::print_hex_padded(val[0], WORD_HEX_DIGITS, HexCase::Lower);
    if opt_words > 1 {
        G::print_str(" OPTCR1: ");
        G::print_hex_padded(val[1], WORD_HEX_DIGITS, HexCase::Lower);
        if opt_words > 2 {
            G::print_str(" OPTCR2: ");
            G::print_hex_padded(val[2], WORD_HEX_DIGITS, HexCase::Lower);
        }
    }
    G::print_str("\n");
    encoder::reply_ok();
    true
}

/// Handle `mon psize` — read or set the flash write parallelism.
pub fn _psize(_command: &str, args: &[&str]) -> bool {
    let Some(info) = driver::active_info() else {
        return false;
    };
    if args.is_empty() {
        G::print_str("Flash write parallelism: ");
        G::print_str(flash::psize_name(info.psize));
        G::print_str("\n");
    } else {
        let Some(psize) = flash::psize_from_name(args[0]) else {
            gdb_println!("usage: monitor psize (x8|x16|x32|x64)");
            encoder::reply_ok();
            return true;
        };
        driver::set_psize(psize);
    }
    encoder::reply_ok();
    true
}

/// Handle `mon uid` — print the factory unique device id.
pub fn _uid(_command: &str, _args: &[&str]) -> bool {
    let Some(info) = driver::active_info() else {
        return false;
    };
    let uid_base = match info.part_id {
        PART_STM32F72X => F72X_UID_BASE,
        PART_STM32F74X | PART_STM32F76X => F7_UID_BASE,
        _ => F4_UID_BASE,
    };
    let cm = CortexMAccess::new(get_cur_target());
    if !print_uid(&cm, uid_base) {
        return false;
    }
    encoder::reply_ok();
    true
}
