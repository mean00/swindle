/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (c) 2021-2023 Fabrice Prost-Boucle <fabalthazar@falbalab.fr>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the `mon` command tree of blackmagic/src/target/stm32g0.c
 * (`stm32g0_cmd_erase_bank()`, `_cmd_option()`, `_cmd_irreversible()`, `_cmd_uid()` and
 * their helpers), made for the Swindle firmware. This file is derived from that C source
 * and stays under the same license as the original Black Magic code reproduced in
 * `lib.rs`.
 */

//! `mon` (GDB `monitor`) commands of the STM32G0/C0/U0 driver.
//!
//! They are declared through `RustArmTargetDriver::commands()` / `command_help()`, so
//! they only exist while a target driven by this crate is attached — same names and
//! semantics as the former C driver:
//!
//! - `mon erase_bank 1|2` — mass erase one flash bank
//! - `mon option` / `mon option erase` / `mon option write <addr> <val> [...]` — dump,
//!   unprotect and program the option registers
//! - `mon irreversible enable|disable` — enable OTP programming and RDP level 2
//! - `mon uid` — dump the factory unique device id
//!
//! The STM32U0 parts only get `mon uid` (C `stm32u0_cmd_list`).

use crate::driver::{
    active_part, irreversible_enabled, set_irreversible_enabled,
};
use crate::flash;
use crate::parts::*;
use crate::registers::*;
use rs_stm32_common::output::{partial_match, print_uid};
use rsbmp::bmplogger::G;
use rsbmp::commands::{CallbackType, CommandTree};
use rsbmp::encoder::encoder;
use rsbmp::format_util::HexCase;
use rsbmp::gdb_println;
use rsbmp::parsing_util::ascii_hex_or_dec_to_u32;
use rsbmp::target::{CortexMAccess, get_cur_target};

/// Commands this driver adds to the generic `mon` tree (C `stm32g0_cmd_list`).
pub const COMMANDS: [CommandTree; 4] = [
    CommandTree {
        command: "erase_bank",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_erase_bank),
        start_separator: b' ',
        next_separator: b' ',
        help: "Erase specified Flash bank",
    },
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
        command: "irreversible",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_irreversible),
        start_separator: b' ',
        next_separator: b' ',
        help: "Allow irreversible operations: (enable|disable)",
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

/// Commands of the STM32U0 parts (C `stm32u0_cmd_list`).
pub const U0_COMMANDS: [CommandTree; 1] = [CommandTree {
    command: "uid",
    min_args: 0,
    require_connected: true,
    cb: CallbackType::text(_uid),
    start_separator: b' ',
    next_separator: b' ',
    help: "Print unique device ID",
}];

/// Handle `mon erase_bank` — mass erase the flash bank given as first argument.
pub fn _erase_bank(_command: &str, args: &[&str]) -> bool {
    let mut bank = 0;
    if args.len() == 1 {
        bank = match args[0].as_bytes().first() {
            Some(b'1') => 1,
            Some(b'2') => 2,
            _ => 0,
        };
    }

    if bank == 0 {
        gdb_println!("Must specify which bank to erase");
        return false;
    }

    let cm = CortexMAccess::new(get_cur_target());
    if !flash::erase_bank(&cm, bank) {
        return false;
    }
    encoder::reply_ok();
    true
}

/// Handle `mon option` — dump, erase or program the option registers.
pub fn _option(_command: &str, args: &[&str]) -> bool {
    let Some(part) = active_part() else {
        return false;
    };
    let cm = CortexMAccess::new(get_cur_target());
    let mut values: [Option<u32>; OPTION_REGISTER_COUNT] = [None; OPTION_REGISTER_COUNT];

    if args.len() == 1 && args[0].eq_ignore_ascii_case("erase") {
        for (index, register) in OPTION_REGISTERS.iter().enumerate() {
            values[index] = Some(register.value);
        }
        // The C0x1 parts have a live `BOREN` bit, so they reset to their own default.
        if is_c0_part(part.id) {
            values[OPT_REG_OPTR] = Some(OPTION_DEFAULT_C0X1);
        }
        if !write_options(&cm, &values) {
            return false;
        }
    } else if args.len() > 1 && args.len() % 2 == 1 && args[0].eq_ignore_ascii_case("write") {
        // `args[1..]` holds the (address, value) pairs the user asked for.
        let mut valid = 0;
        for pair in args[1..].chunks(2) {
            let addr = ascii_hex_or_dec_to_u32(pair[0]);
            let value = ascii_hex_or_dec_to_u32(pair[1]);
            if add_reg_value(&mut values, addr, value) {
                valid += 1;
            }
        }
        if valid == 0 || !validate_options(&values) || !write_options(&cm, &values) {
            return false;
        }
    } else {
        gdb_println!("usage: monitor option erase");
        gdb_println!("usage: monitor option write <addr> <val> [<addr> <val>]...");
        display_registers(&cm);
    }

    encoder::reply_ok();
    true
}

/// Handle `mon irreversible` — allow or forbid OTP programming and RDP level 2.
pub fn _irreversible(_command: &str, args: &[&str]) -> bool {
    let mut enabled = irreversible_enabled();
    // Without an argument the command just reports the current state.
    let parsed = if args.len() == 1 {
        parse_enable_or_disable(args[0], &mut enabled)
    } else {
        true
    };
    set_irreversible_enabled(enabled);
    gdb_println!(
        "Irreversible operations: ",
        if enabled { "enabled" } else { "disabled" }
    );
    parsed
}

/// Handle `mon uid` — print the factory unique device id of the part.
pub fn _uid(_command: &str, _args: &[&str]) -> bool {
    let Some(part) = active_part() else {
        return false;
    };
    let uid_base = match part.id {
        PART_STM32C011 | PART_STM32C031 | PART_STM32C051 | PART_STM32C071 | PART_STM32C09X => {
            C0_UID_BASE
        }
        PART_STM32U031 => U031_UID_BASE,
        PART_STM32U0X3 => U0X3_UID_BASE,
        _ => G0_UID_BASE,
    };
    let cm = CortexMAccess::new(get_cur_target());
    if !print_uid(&cm, uid_base) {
        return false;
    }
    encoder::reply_ok();
    true
}

/// Whether `part_id` is one of the STM32C0x1 parts (C lists them one by one).
fn is_c0_part(part_id: u16) -> bool {
    matches!(
        part_id,
        PART_STM32C011 | PART_STM32C031 | PART_STM32C051 | PART_STM32C071 | PART_STM32C09X
    )
}

/// Parse an `enable`/`disable` argument; `false` after telling the user what it wants.
fn parse_enable_or_disable(value: &str, out: &mut bool) -> bool {
    if !value.is_empty() && partial_match(value, "enable") {
        *out = true;
    } else if !value.is_empty() && partial_match(value, "disable") {
        *out = false;
    } else {
        gdb_println!("'enable' or 'disable' argument must be provided");
        return false;
    }
    true
}

/// Add the `(addr, value)` pair to `values`; `false` when `addr` is not an option register.
fn add_reg_value(
    values: &mut [Option<u32>; OPTION_REGISTER_COUNT],
    addr: u32,
    value: u32,
) -> bool {
    for (index, register) in OPTION_REGISTERS.iter().enumerate() {
        if register.addr == addr {
            values[index] = Some(value);
            return true;
        }
    }
    false
}

/// Only allow read protection level 2 when the user asked for irreversible operations.
fn validate_options(values: &[Option<u32>; OPTION_REGISTER_COUNT]) -> bool {
    // An `OPTR` the user did not name stays at 0, which is level 0.
    let optr = values[OPT_REG_OPTR].unwrap_or(0);
    let valid = optr & FPEC_OPTION_RDP_MASK != FPEC_OPTION_RDP_LEVEL2 || irreversible_enabled();
    if !valid {
        gdb_println!("Irreversible operations disabled");
    }
    valid
}

/// Program `values` into the option registers, reporting a failure like the C driver.
fn write_options(cm: &CortexMAccess, values: &[Option<u32>; OPTION_REGISTER_COUNT]) -> bool {
    if !flash::option_write(cm, values) {
        gdb_println!("Writing options failed!");
        return false;
    }
    true
}

/// Print every option register with its current value (C `_display_registers()`).
fn display_registers(cm: &CortexMAccess) {
    for register in OPTION_REGISTERS.iter() {
        let value = cm.read32(register.addr);
        // C prints both fields with `0x%08" PRIX32`.
        G::print_str("0x");
        G::print_hex_padded(register.addr, ADDR_HEX_DIGITS, HexCase::Upper);
        G::print_str(": 0x");
        G::print_hex_padded(value, WORD_HEX_DIGITS, HexCase::Upper);
        G::print_str("\n");
    }
}
