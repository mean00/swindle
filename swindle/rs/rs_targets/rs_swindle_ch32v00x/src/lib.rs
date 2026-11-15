/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2022-2026 1BitSquared <info@1bitsquared.com>
 * Written by Rafael Silva <perigoso@riseup.net>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of `blackmagic/src/target/ch32vx.c` (`ch32v003x_probe()` and the CH32V00x
 * driver), made for the Swindle firmware. This file is derived from that C source and
 * stays under the same license as the original Black Magic code reproduced below.
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

#![no_std]

pub mod flashstub;
pub mod registers;
pub mod driver;

pub use flashstub::*;
pub use registers::*;
pub use driver::*;
