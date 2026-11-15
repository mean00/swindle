/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (c) 2021-2023 Fabrice Prost-Boucle <fabalthazar@falbalab.fr>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of blackmagic/src/target/stm32g0.c (the STM32G0/STM32C0/STM32U0 part
 * table, memory map, flash programming routines and `mon` commands), made for the
 * Swindle firmware. This file is derived from that C source and stays under the same
 * license as the original Black Magic code reproduced below.
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

#![no_std]

//! Pure Rust driver for the STM32G0, STM32C0 and STM32U0 families.
//!
//! These parts share the second generation of the STM32 flash controller: pages of
//! 2 KiB, an explicit bank selector (`BKER`) for the dual-bank parts, option registers
//! written through `OPTSTART` + `OBL_LAUNCH`, and a `DBGMCU` block that only answers
//! once its RCC clock gate is open. The driver here mirrors `stm32g0.c` one to one:
//! [`parts`] holds the part table, [`flash`] the controller code, [`driver`] the
//! `RustArmTargetDriver` implementation and [`mon_commands`] the `mon` commands.
//!
//! The crate is linked into `rs_swindle_wrapper`, which exports `stm32g0_probe` to the
//! C target layer and drives the resulting driver through `rsbmp::target`.

pub mod driver;
pub mod flash;
pub mod mon_commands;
pub mod parts;
pub mod registers;

pub use driver::G0Driver;
