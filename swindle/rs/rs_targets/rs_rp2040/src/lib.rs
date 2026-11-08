/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2021 Uwe Bonnes (bon@elektron.ikp.physik.tu-darmstadt.de)
 * Copyright (C) 2022-2023 1BitSquared <info@1bitsquared.com>
 * Significantly rewritten in 2022 by Rachel Mant <git@dragonmux.network>
 * Copyright (C) 2022 James Turton
 * Includes extracts from pico-bootrom
 * Copyright (C) 2020 Raspberry Pi (Trading) Ltd
 *
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice, this
 *    list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 * 3. Neither the name of the copyright holder nor the names of its
 *    contributors may be used to endorse or promote products derived from this
 *    software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
 * LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */

//! Pure Rust driver for the RP2040 (Raspberry Pi Pico 1).
//!
//! Rust port of `blackmagic/src/target/rp2040.c`, made for the Swindle firmware. The
//! crate is linked into `rs_swindle_wrapper`, which exposes [`driver::probe`] to the C
//! target layer as `rp2040_probe` and drives the result through
//! `rsbmp::target::RustArmTargetDriver`.
//!
//! The flash chip behind the QSPI pads is handled by the shared layer in
//! `rs_rp_common` (ported `spi.c`/`sfdp.c`); this crate supplies the SSI side of it, the
//! XIP entry/exit sequences and the SRAM stub that does the page programming.
//!
//! The ADIv5 rescue path (`rp2040_rescue_probe()`, `rp2040_rescue_do_reset()`) lives in
//! [`rescue`]. With it ported, C has no `rp2040.c` left and the `bmp_spi_add_flash()` manager
//! (`spi.c`/`sfdp.c`) goes with it — the QSPI side is the `rs_rp_common` port of the same
//! files.

#![no_std]

pub mod crc32;
pub mod driver;
pub mod flash;
pub mod flashstub_write;
pub mod mon_commands;
pub mod registers;
pub mod rescue;

pub use driver::Rp2040Driver;
