/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2017, 2018 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (c) 2021-2023 Fabrice Prost-Boucle <fabalthazar@falbalab.fr>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the `mon uid` output helpers of blackmagic/src/target/stm32_common.c
 * (`stm32_uid()`, used by stm32f1.c, stm32f4.c and stm32g0.c), made for the Swindle
 * firmware. This file is derived from that C source and stays under the same licenses as
 * the original Black Magic code reproduced in `lib.rs`.
 */

//! Output helpers shared by the `mon` commands of the STM32 drivers.
//!
//! GDB's `O`/`E` packets carry raw text, so everything here writes through
//! `bmplogger::G`; the hex digits themselves come from `rsbmp::format_util`.

use crate::registers::{UID_LEN, UID_WORD_HEX_DIGITS};
use rsbmp::bmplogger::G;
use rsbmp::encoder::encoder;
use rsbmp::format_util::HexCase;
use rsbmp::gdb_println;
use rsbmp::target::CortexMAccess;

// Byte offsets of the fields of C `stm32_uid_s` inside the UID block.
const UID_WAFER_X: usize = 0;
const UID_WAFER_Y: usize = 2;
const UID_WAFER_NUMBER: usize = 4;
const UID_LOT_NUMBER: usize = 5;

/// Print the factory UID at `base`: one `0x`-prefixed line of `UID_LEN * 2`
/// upper-case digits, plus the decoded wafer/lot line when the C driver prints it.
///
/// `false` after reporting a read failure to GDB (the caller must not add a reply).
pub fn print_uid(cm: &CortexMAccess, base: u32) -> bool {
    let mut uid = [0u8; UID_LEN];
    cm.mem_read(&mut uid, base);
    if cm.check_error() {
        gdb_println!("UID read failed");
        encoder::reply_e01();
        return false;
    }
    // Same rendering as the C `stm32_uid()`: one `0x`, then each 32-bit word as
    // eight upper-case digits, the words read as big-endian so that the first
    // byte of the UID comes out first.
    G::print_str("0x");
    for word in 0..UID_LEN / 4 {
        let bytes = &uid[word * 4..word * 4 + 4];
        let value = u32::from_be_bytes([bytes[3], bytes[2], bytes[1], bytes[0]]);
        G::print_hex_digits(value, UID_WORD_HEX_DIGITS, HexCase::Upper);
    }
    G::print_str("\n");
    print_wafer_line(&uid);
    true
}

/// Print C `stm32_uid()`'s `Wafer coords ...` line.
///
/// The C driver only decodes it when every lot-number byte is printable ASCII, and
/// so does this port.
fn print_wafer_line(uid: &[u8; UID_LEN]) {
    let lot_number = &uid[UID_LOT_NUMBER..UID_LEN];
    if lot_number.iter().any(|&byte| !(0x20..0x7f).contains(&byte)) {
        return;
    }
    let wafer_x = u16::from_le_bytes([uid[UID_WAFER_X], uid[UID_WAFER_X + 1]]);
    let wafer_y = u16::from_le_bytes([uid[UID_WAFER_Y], uid[UID_WAFER_Y + 1]]);
    G::print_str("Wafer coords X=");
    G::print_int(i64::from(wafer_x));
    G::print_str(", Y=");
    G::print_int(i64::from(wafer_y));
    G::print_str(", number ");
    G::print_int(i64::from(uid[UID_WAFER_NUMBER]));
    G::print_str("; Lot number ");
    // The range check above guarantees this is ASCII.
    G::print_str(core::str::from_utf8(lot_number).unwrap_or(""));
    G::print_str("\n");
}

/// Case-insensitive prefix match, as the C driver spells `partial_match()`: `given`
/// may be a prefix of `command`, never longer.
pub fn partial_match(given: &str, command: &str) -> bool {
    given.len() <= command.len() && command[..given.len()].eq_ignore_ascii_case(given)
}
