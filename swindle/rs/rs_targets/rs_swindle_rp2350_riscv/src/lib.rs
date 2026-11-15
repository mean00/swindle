/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2024 1BitSquared <info@1bitsquared.com>
 * Written by Rachel Mant <git@dragonmux.network>
 * All rights reserved.
 *
 * Rust port of the Hazard3 half of `blackmagic/src/target/rp2350.c`, made for the Swindle
 * firmware. This file is derived from that C source and stays under the same license as
 * the original Black Magic code reproduced below.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice, this
 *    list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * 3. Neither the name of the copyright holder nor the names of its
 *    contributors may be used to endorse or promote products derived from
 *    this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

//! The RP2350 driver for its Hazard3 RISC-V cores.
//!
//! The RISC-V half of `blackmagic/src/target/rp2350.c`: `rp2350_probe()` for a Hazard3 hart,
//! the memory map, the flash hooks and the DMA-sniffer CRC32, on top of the flash controller
//! in [`rs_swindle_rp2350_common`]. The Arm cores of the same chip are served by `rs_swindle_rp2350_arm`;
//! which of the two a probe reaches is decided by the designer code the RISC-V scan found
//! (see `rs_swindle_wrapper/src/riscv_shim.rs`).
//!
//! Unlike the Cortex-M side, a hart is reached through
//! [`rs_swindle_rp_common::access::HartAccess`], so the shared flash code drives both.

#![no_std]

pub mod crc32;
pub mod driver;

pub use driver::Rp2350RiscvDriver;
