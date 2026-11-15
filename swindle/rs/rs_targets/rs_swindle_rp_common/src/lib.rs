/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2023-2024 1BitSquared <info@1bitsquared.com>
 * Written by Rachel Mant <git@dragonmux.network>
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

//! What the Raspberry Pi microcontroller drivers share: the SPI NOR flash layer.
//!
//! Rust port of `blackmagic/src/target/spi.c` and `sfdp.c`, the parts the RP2040
//! (SSI controller) and RP2350 (QMI controller) drivers of this workspace need. The
//! bus itself is not here: a driver implements [`spi::SpiTransport`] with its own
//! controller (SSI or QMI) and the flash layer then talks to the chip through it, the
//! way the C `spi_read_func`/`spi_write_func`/`spi_run_command_func` trio is passed to
//! `bmp_spi_add_flash()`.
//!
//! The other C SPI flash users (LPC43xx, i.MXRT, ECP5, RZ, onboard flash) keep the C
//! layer, so nothing here is shared with them.
//!
//! The bus is reached through [`access::FlashAccess`], the small target-side API the flash
//! layer needs (word and buffer access, the fault flag, the reset after reprogramming), so
//! the same code drives a Cortex-M core ([`rsbmp::target::CortexMAccess`]) and a RISC-V hart
//! ([`access::HartAccess`]).
//!
//! [`flashstub_crc32`] holds the DMA sniffer CRC32 stub the RP2040 and RP2350 drivers both
//! park in SRAM for the `qCRC` fast path.

#![no_std]

pub mod access;
pub mod flashstub_crc32;
pub mod sfdp;
pub mod spi;
