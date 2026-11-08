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
 * Rust port of the parts of blackmagic/src/target/stm32f1.c, stm32f4.c (GPL-3.0-or-later)
 * and stm32g0.c (MIT, see the notice below and the ported G0 driver) that the STM32 family
 * drivers share: the debug-side core (DBGMCU clock gate, freeze bits, attach/detach), the
 * `mon` output helpers and the constants every family register map repeats. Made for the
 * Swindle firmware; derived from those C sources under the same licenses as the original
 * Black Magic code reproduced below.
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
 *
 * The MIT-licensed parts (stm32g0.c) are:
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

#![no_std]

//! Pieces shared by the Rust STM32 family drivers (`rs_stm32f1f4`, `rs_stm32g0`).
//!
//! The families differ in their flash controllers and memory maps, but the debug-side
//! setup is the same shape everywhere: enable the DBGMCU clock when it sits behind a
//! gate, set the freeze bits so the watchdogs and the low-power modes cannot take the
//! core away from the debugger, and undo exactly that on detach. [`driver::Stm32Driver`]
//! keeps that list of registers as data, so each family publishes its own table instead
//! of its own copy of the sequence.

pub mod dbgmcu;
pub mod driver;
pub mod output;
pub mod registers;

pub use driver::{DbgClockGate, FreezeBits, Stm32Driver};
