/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2023 1BitSquared <info@1bitsquared.com>
 * Written by Rachel Mant <git@dragonmux.network>
 * All rights reserved.
 *
 * Rust port of the CSR, `dcsr` and `misa` definitions of
 * `blackmagic/src/target/riscv32.c`, made for the Swindle firmware. This file is derived
 * from that C source and stays under the same license as the original Black Magic code
 * reproduced below.
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

//! RISC-V register numbering: CSR addresses, debug CSR (`dcsr`) bits, `misa`
//! extension bits, abstract register-file bases and the GDB register map.
//!
//! These are pure spec constants shared by `hart.rs` (run control, register
//! access), `stub.rs` (on-target program register use), `commands/mon.rs`
//! (`mon csr <name>`) and the FreeRTOS thread switchers. Nothing here depends
//! on a transport or on the debug module, so any layer may use them.

// --- CSR Addresses ---
pub const RV_CSR_FFLAGS: u16 = 0x001;
pub const RV_CSR_FRM: u16 = 0x002;
pub const RV_CSR_FCSR: u16 = 0x003;
pub const RV_CSR_MSTATUS: u16 = 0x300;
pub const RV_CSR_MISA: u16 = 0x301;
pub const RV_CSR_MIE: u16 = 0x304;
pub const RV_CSR_MTVEC: u16 = 0x305;
pub const RV_CSR_MSCRATCH: u16 = 0x340;
pub const RV_CSR_MEPC: u16 = 0x341;
pub const RV_CSR_MCAUSE: u16 = 0x342;
pub const RV_CSR_MTVAL: u16 = 0x343;
pub const RV_CSR_MIP: u16 = 0x344;
pub const RV_CSR_TSELECT: u16 = 0x7a0;
pub const RV_CSR_TDATA1: u16 = 0x7a1;
pub const RV_CSR_TDATA2: u16 = 0x7a2;
pub const RV_CSR_TDATA3: u16 = 0x7a3;
pub const RV_CSR_TINFO: u16 = 0x7a4;
pub const RV_CSR_DCSR: u16 = 0x7b0;
pub const RV_CSR_DPC: u16 = 0x7b1;
pub const RV_CSR_DSCRATCH0: u16 = 0x7b2;
pub const RV_CSR_DSCRATCH1: u16 = 0x7b3;
pub const RV_CSR_MVENDORID: u16 = 0xf11;
pub const RV_CSR_MARCHID: u16 = 0xf12;
pub const RV_CSR_MIMPID: u16 = 0xf13;
pub const RV_CSR_MHARTID: u16 = 0xf14;

// --- Register bases in abstract register space ---
pub const RV_REG_GPR_BASE: u16 = 0x1000;
pub const RV_REG_FPR_BASE: u16 = 0x1020;

// --- GPR numbers (abstract register space, RV32E/I ABI) ---
pub const RV_REG_SP: u8 = 2;
pub const RV_REG_A0: u8 = 10;
pub const RV_REG_A1: u8 = 11;
pub const RV_REG_A2: u8 = 12;
pub const RV_REG_A3: u8 = 13;

// --- dcsr bits ---
pub const RV_DCSR_STEP: u32 = 1 << 2;
pub const RV_DCSR_CAUSE_SHIFT: u32 = 6;
pub const RV_DCSR_CAUSE_MASK: u32 = 0x7 << 6;
pub const RV_DCSR_EBREAKM: u32 = 1 << 15;

// --- misa extension bits ---
pub const RV_MISA_EXT_E: u32 = 1 << 4;
pub const RV_MISA_EXT_F: u32 = 1 << 5;
pub const RV_MISA_EXT_I: u32 = 1 << 8;

// --- GDB register numbering ---
/// GDB numbers CSR `n` as `RV_CSR_GDB_OFFSET + n`, which puts the CSR block
/// above the GPR (0..32) and FPU (33..68) registers. Requested CSRs start at
/// 128 (= `+ mstatus`, 0x300), so GDB numbers 69..127 stay unused.
pub const RV_CSR_GDB_OFFSET: u32 = 128;
/// GDB number of `f0` (single-precision FPU register file).
pub const RV_FPU_GDB_OFFSET: u32 = 33;
/// GDB number of `fflags` (FPU CSRs follow `frm`, `fcsr`).
pub const RV_FPU_GDB_CSR_OFFSET: u32 = 66;
