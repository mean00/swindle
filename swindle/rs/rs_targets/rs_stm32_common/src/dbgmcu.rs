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
 * Rust port of the DBGMCU register access pattern of blackmagic/src/target/stm32f1.c,
 * stm32f4.c and stm32g0.c (`read/modify/write` on the debug MCU block, and the
 * `IDCODE` decoding the probes use), made for the Swindle firmware. This file is
 * derived from those C sources and stays under the same licenses as the original Black
 * Magic code reproduced in `lib.rs`.
 */

//! Read/modify/write helpers for the DBGMCU block.
//!
//! Every family configures its debug block the same way: read a register, OR in (or AND
//! out) a few bits, write it back. Only the addresses and bits differ, which is what
//! [`crate::driver::Stm32Driver`] stores.

use crate::registers::IDCODE_DEV_ID_MASK;
use rsbmp::target::CortexMAccess;

/// Set `bits` of the register at `addr`, leaving the rest untouched.
pub fn set_bits(cm: &CortexMAccess, addr: u32, bits: u32) {
    cm.write32(addr, cm.read32(addr) | bits);
}

/// Clear `bits` of the register at `addr`, leaving the rest untouched.
pub fn clear_bits(cm: &CortexMAccess, addr: u32, bits: u32) {
    cm.write32(addr, cm.read32(addr) & !bits);
}

/// The `DEV_ID` field of the DBGMCU `IDCODE` register (`0xfff` wide on every family).
pub fn idcode_dev_id(cm: &CortexMAccess, addr: u32) -> u16 {
    (cm.read32(addr) & IDCODE_DEV_ID_MASK) as u16
}
