/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2023-2024 1BitSquared <info@1bitsquared.com>
 * Written by Rachel Mant <git@dragonmux.network>
 * All rights reserved.
 *
 * Rust conversion of blackmagic/src/target/riscv32.c and
 * blackmagic/src/target/riscv_debug.c, made for the Swindle firmware. This file
 * is a translation of those C sources into Rust and stays under the same license
 * as the original Black Magic code reproduced below.
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

//! Memory access dispatch (Sysbus, Abstractauto streaming, Progbuf, and SDI retry).

use super::dm::*;
use super::dm_regs::*;
use super::dmi::{DmiError, DmiTransport};
use super::hart::{MemoryAccessMethod, RiscvHart};
use super::rv_csr::{RV_CSR_DPC, RV_REG_A0, RV_REG_A1, RV_REG_GPR_BASE};
use crate::target::driver::TargetError;

// Sysbus Control Bits
pub const RV_SBCS_SBACCESS_8: u32 = 0 << 17;
pub const RV_SBCS_SBACCESS_16: u32 = 1 << 17;
pub const RV_SBCS_SBACCESS_32: u32 = 2 << 17;
pub const RV_SBCS_SBAUTOINCREMENT: u32 = 1 << 16;
pub const RV_SBCS_SBREADONDATA: u32 = 1 << 15;
pub const RV_SBCS_SBREADONADDR: u32 = 1 << 20;
pub const RV_SBCS_SBBUSY: u32 = 1 << 21;
pub const RV_SBCS_SBBUSYERROR: u32 = 1 << 22;
pub const RV_SBCS_SBERROR_MASK: u32 = 0x7 << 12;

// --- RV32I encodings for the program buffer accessor ---
//
// Uncompressed loads/stores through the address in a0, with a1 carrying the
// data, exactly as the legacy C accessor encoded them (`riscv32.c`).
const RV_LOAD_BYTE: u32 = 0x0000_0003;
const RV_LOAD_HALF: u32 = 0x0000_1003;
const RV_LOAD_WORD: u32 = 0x0000_2003;
const RV_STORE_BYTE: u32 = 0x0000_0023;
const RV_STORE_HALF: u32 = 0x0000_1023;
const RV_STORE_WORD: u32 = 0x0000_2023;
/// `rs1 = a0` (the address the load/store uses)
const RV_RS1_A0: u32 = 0x0005_0000;
/// `rs2 = a1` (the store's data)
const RV_RS2_A1: u32 = 0x00b0_0000;
/// `rd = a1` (the load's destination)
const RV_RD_A1: u32 = 0x0000_0580;
/// `ebreak`: a program buffer run shorter than the buffer has to end in one.
const RV_EBREAK: u32 = 0x0010_0073;

/// Biggest block one `RV.MEM_READ` / `RV.MEM_WRITE` frame carries (hosted bulk access).
///
/// It is the single source of the block size: the host splits transfers with it
/// (`hosted/bulk_mem.rs`), the probe refuses anything bigger (`rpc_rv_impl.rs`), and
/// both sides assert at compile time that their buffers hold a full block.
pub const MEM_BLOCK_MAX: usize = 1024;

/// Memory access dispatcher for RISC-V targets.
pub struct RiscvMem;

impl RiscvMem {
    /// Read arbitrary memory bytes from `addr` into `dest`.
    pub fn read(
        hart: &mut RiscvHart,
        addr: u32,
        dest: &mut [u8],
    ) -> Result<(), TargetError> {
        Self::read_with(&mut hart.dmi, hart.active_method, addr, dest)
    }

    /// [`Self::read`] on any DMI transport: the dispatch the hosted probe runs for a
    /// `RV.MEM_READ` block is this very function, so both ends of the link stay identical.
    pub(crate) fn read_with(
        dmi: &mut dyn DmiTransport,
        method: MemoryAccessMethod,
        addr: u32,
        dest: &mut [u8],
    ) -> Result<(), TargetError> {
        if dest.is_empty() {
            return Ok(());
        }

        // Bounded retry loop for SDI read confirmation (patch 102 & 103)
        let retry_count = if dmi.needs_read_confirm() { 4 } else { 1 };
        for _ in 0..retry_count {
            if Self::read_attempt(dmi, method, addr, dest).is_ok() {
                return Ok(());
            }
        }
        Err(TargetError::AccessFault)
    }

    fn read_attempt(
        dmi: &mut dyn DmiTransport,
        method: MemoryAccessMethod,
        addr: u32,
        dest: &mut [u8],
    ) -> Result<(), TargetError> {
        match method {
            MemoryAccessMethod::Sysbus => Self::sysbus_read(dmi, addr, dest),
            MemoryAccessMethod::AbstAuto => {
                if dest.len() <= 16 {
                    Self::abstract_read(dmi, addr, dest)
                } else {
                    let mut cur_addr = addr;
                    let mut remaining = dest;
                    let unaligned = (cur_addr & 3) as usize;
                    if unaligned != 0 {
                        let leading = core::cmp::min(4 - unaligned, remaining.len());
                        Self::abstract_read(dmi, cur_addr, &mut remaining[..leading])?;
                        cur_addr += leading as u32;
                        remaining = &mut remaining[leading..];
                    }
                    let aligned_len = remaining.len() & !3;
                    if aligned_len >= 16 {
                        Self::abstract_auto_read(dmi, cur_addr, &mut remaining[..aligned_len])?;
                        cur_addr += aligned_len as u32;
                        remaining = &mut remaining[aligned_len..];
                    }
                    if !remaining.is_empty() {
                        Self::abstract_read(dmi, cur_addr, remaining)?;
                    }
                    Ok(())
                }
            }
            MemoryAccessMethod::Abstract => Self::abstract_read(dmi, addr, dest),
            MemoryAccessMethod::Progbuf => Self::progbuf_read(dmi, addr, dest),
        }
    }

    /// Write arbitrary memory bytes from `src` to `addr`.
    pub fn write(
        hart: &mut RiscvHart,
        addr: u32,
        src: &[u8],
    ) -> Result<(), TargetError> {
        Self::write_with(&mut hart.dmi, hart.active_method, addr, src)
    }

    /// [`Self::write`] on any DMI transport (see [`Self::read_with`]).
    pub(crate) fn write_with(
        dmi: &mut dyn DmiTransport,
        method: MemoryAccessMethod,
        addr: u32,
        src: &[u8],
    ) -> Result<(), TargetError> {
        if src.is_empty() {
            return Ok(());
        }
        match method {
            MemoryAccessMethod::Sysbus => Self::sysbus_write(dmi, addr, src),
            MemoryAccessMethod::AbstAuto => {
                if src.len() <= 16 {
                    Self::abstract_write(dmi, addr, src)
                } else {
                    let mut cur_addr = addr;
                    let mut remaining = src;
                    let unaligned = (cur_addr & 3) as usize;
                    if unaligned != 0 {
                        let leading = core::cmp::min(4 - unaligned, remaining.len());
                        Self::abstract_write(dmi, cur_addr, &remaining[..leading])?;
                        cur_addr += leading as u32;
                        remaining = &remaining[leading..];
                    }
                    let aligned_len = remaining.len() & !3;
                    if aligned_len >= 16 {
                        Self::abstract_auto_write(dmi, cur_addr, &remaining[..aligned_len])?;
                        cur_addr += aligned_len as u32;
                        remaining = &remaining[aligned_len..];
                    }
                    if !remaining.is_empty() {
                        Self::abstract_write(dmi, cur_addr, remaining)?;
                    }
                    Ok(())
                }
            }
            MemoryAccessMethod::Abstract => Self::abstract_write(dmi, addr, src),
            MemoryAccessMethod::Progbuf => Self::progbuf_write(dmi, addr, src),
        }
    }

    /// Non-halting read via Sysbus (returns true if performed).
    pub fn read_nostop(
        hart: &mut RiscvHart,
        addr: u32,
        dest: &mut [u8],
    ) -> Result<bool, TargetError> {
        if hart.has_sysbus {
            Self::sysbus_read(&mut hart.dmi, addr, dest).map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Non-halting write via Sysbus (returns true if performed).
    pub fn write_nostop(
        hart: &mut RiscvHart,
        addr: u32,
        src: &[u8],
    ) -> Result<bool, TargetError> {
        if hart.has_sysbus {
            Self::sysbus_write(&mut hart.dmi, addr, src).map(|_| true)
        } else {
            Ok(false)
        }
    }

    // --- Sysbus Operations ---

    fn wait_sysbus_idle(dmi: &mut dyn DmiTransport) -> Result<(), DmiError> {
        for _ in 0..1000 {
            let cs = dmi.read(RV_DM_SBCS)?;
            if (cs & (RV_SBCS_SBBUSY | RV_SBCS_SBBUSYERROR)) == 0 {
                let err = (cs & RV_SBCS_SBERROR_MASK) >> 12;
                if err != 0 {
                    // Clear error by writing 1 to the error bits
                    let _ = dmi.write(RV_DM_SBCS, cs);
                    return Err(DmiError::Fault);
                }
                return Ok(());
            }
        }
        Err(DmiError::Timeout)
    }

    pub(crate) fn sysbus_read(dmi: &mut dyn DmiTransport, mut addr: u32, mut dest: &mut [u8]) -> Result<(), TargetError> {
        // Word-aligned chunk loop
        while dest.len() >= 4 && (addr & 3) == 0 {
            let sbcs = RV_SBCS_SBACCESS_32 | RV_SBCS_SBAUTOINCREMENT | RV_SBCS_SBREADONADDR;
            dmi.write(RV_DM_SBCS, sbcs).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBADDRESS0, addr).map_err(|_| TargetError::AccessFault)?;
            Self::wait_sysbus_idle(dmi).map_err(|_| TargetError::AccessFault)?;

            let val = dmi.read(RV_DM_SBDATA0).map_err(|_| TargetError::AccessFault)?;
            dest[..4].copy_from_slice(&val.to_le_bytes());
            addr += 4;
            dest = &mut dest[4..];
        }

        // Halfword-aligned loop (crucial for flash programming)
        while dest.len() >= 2 && (addr & 1) == 0 {
            let sbcs = RV_SBCS_SBACCESS_16 | RV_SBCS_SBREADONADDR;
            dmi.write(RV_DM_SBCS, sbcs).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBADDRESS0, addr).map_err(|_| TargetError::AccessFault)?;
            Self::wait_sysbus_idle(dmi).map_err(|_| TargetError::AccessFault)?;

            let val = dmi.read(RV_DM_SBDATA0).map_err(|_| TargetError::AccessFault)? as u16;
            dest[..2].copy_from_slice(&val.to_le_bytes());
            addr += 2;
            dest = &mut dest[2..];
        }

        // Byte-wise remainder
        while !dest.is_empty() {
            let sbcs = RV_SBCS_SBACCESS_8 | RV_SBCS_SBREADONADDR;
            dmi.write(RV_DM_SBCS, sbcs).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBADDRESS0, addr).map_err(|_| TargetError::AccessFault)?;
            Self::wait_sysbus_idle(dmi).map_err(|_| TargetError::AccessFault)?;

            let val = dmi.read(RV_DM_SBDATA0).map_err(|_| TargetError::AccessFault)? as u8;
            dest[0] = val;
            addr += 1;
            dest = &mut dest[1..];
        }

        Ok(())
    }

    pub(crate) fn sysbus_write(dmi: &mut dyn DmiTransport, mut addr: u32, mut src: &[u8]) -> Result<(), TargetError> {
        // Word-aligned chunk loop
        while src.len() >= 4 && (addr & 3) == 0 {
            let val = u32::from_le_bytes(src[..4].try_into().unwrap());
            let sbcs = RV_SBCS_SBACCESS_32 | RV_SBCS_SBAUTOINCREMENT;
            dmi.write(RV_DM_SBCS, sbcs).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBADDRESS0, addr).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBDATA0, val).map_err(|_| TargetError::AccessFault)?;
            Self::wait_sysbus_idle(dmi).map_err(|_| TargetError::AccessFault)?;

            addr += 4;
            src = &src[4..];
        }

        // Halfword-aligned loop (crucial for flash programming)
        while src.len() >= 2 && (addr & 1) == 0 {
            let val = u16::from_le_bytes(src[..2].try_into().unwrap()) as u32;
            let sbcs = RV_SBCS_SBACCESS_16;
            dmi.write(RV_DM_SBCS, sbcs).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBADDRESS0, addr).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBDATA0, val).map_err(|_| TargetError::AccessFault)?;
            Self::wait_sysbus_idle(dmi).map_err(|_| TargetError::AccessFault)?;

            addr += 2;
            src = &src[2..];
        }

        // Byte-wise remainder
        while !src.is_empty() {
            let val = src[0] as u32;
            let sbcs = RV_SBCS_SBACCESS_8;
            dmi.write(RV_DM_SBCS, sbcs).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBADDRESS0, addr).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBDATA0, val).map_err(|_| TargetError::AccessFault)?;
            Self::wait_sysbus_idle(dmi).map_err(|_| TargetError::AccessFault)?;

            addr += 1;
            src = &src[1..];
        }

        Ok(())
    }

    // --- Abstract Memory Operations ---

    pub(crate) fn abstract_read(
        dmi: &mut dyn DmiTransport,
        mut addr: u32,
        mut dest: &mut [u8],
    ) -> Result<(), TargetError> {
        // For abstract command memory access, cmdtype = 2 (access memory)
        // cmd = (2 << 24) | (aarsize << 20) | (aampostinc << 19)
        while dest.len() >= 4 && (addr & 3) == 0 {
            dmi.write(RV_DM_DATA1, addr).map_err(|_| TargetError::AccessFault)?;
            let cmd = (2 << 24) | (2 << 20) | (1 << 19); // 32-bit, postincrement
            exec_command(dmi, cmd).map_err(|_| TargetError::AccessFault)?;
            let val = dmi.read(RV_DM_DATA0).map_err(|_| TargetError::AccessFault)?;
            dest[..4].copy_from_slice(&val.to_le_bytes());
            addr += 4;
            dest = &mut dest[4..];
        }

        while dest.len() >= 2 && (addr & 1) == 0 {
            dmi.write(RV_DM_DATA1, addr).map_err(|_| TargetError::AccessFault)?;
            let cmd = (2 << 24) | (1 << 20) | (1 << 19); // 16-bit, postincrement
            exec_command(dmi, cmd).map_err(|_| TargetError::AccessFault)?;
            let val = dmi.read(RV_DM_DATA0).map_err(|_| TargetError::AccessFault)? as u16;
            dest[..2].copy_from_slice(&val.to_le_bytes());
            addr += 2;
            dest = &mut dest[2..];
        }

        while !dest.is_empty() {
            dmi.write(RV_DM_DATA1, addr).map_err(|_| TargetError::AccessFault)?;
            let cmd = (2 << 24) | (0 << 20) | (1 << 19); // 8-bit, postincrement
            exec_command(dmi, cmd).map_err(|_| TargetError::AccessFault)?;
            let val = dmi.read(RV_DM_DATA0).map_err(|_| TargetError::AccessFault)? as u8;
            dest[0] = val;
            addr += 1;
            dest = &mut dest[1..];
        }

        dmi.quiesce();
        Ok(())
    }

    pub(crate) fn abstract_write(
        dmi: &mut dyn DmiTransport,
        mut addr: u32,
        mut src: &[u8],
    ) -> Result<(), TargetError> {
        while src.len() >= 4 && (addr & 3) == 0 {
            let val = u32::from_le_bytes(src[..4].try_into().unwrap());
            dmi.write(RV_DM_DATA0, val).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_DATA1, addr).map_err(|_| TargetError::AccessFault)?;
            let cmd = (2 << 24) | (1 << 16) | (2 << 20) | (1 << 19); // write, 32-bit, postincrement
            exec_command(dmi, cmd).map_err(|_| TargetError::AccessFault)?;
            addr += 4;
            src = &src[4..];
        }

        while src.len() >= 2 && (addr & 1) == 0 {
            let val = u16::from_le_bytes(src[..2].try_into().unwrap()) as u32;
            dmi.write(RV_DM_DATA0, val).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_DATA1, addr).map_err(|_| TargetError::AccessFault)?;
            let cmd = (2 << 24) | (1 << 16) | (1 << 20) | (1 << 19); // write, 16-bit, postincrement
            exec_command(dmi, cmd).map_err(|_| TargetError::AccessFault)?;
            addr += 2;
            src = &src[2..];
        }

        while !src.is_empty() {
            let val = src[0] as u32;
            dmi.write(RV_DM_DATA0, val).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_DATA1, addr).map_err(|_| TargetError::AccessFault)?;
            let cmd = (2 << 24) | (1 << 16) | (0 << 20) | (1 << 19); // write, 8-bit, postincrement
            exec_command(dmi, cmd).map_err(|_| TargetError::AccessFault)?;
            addr += 1;
            src = &src[1..];
        }

        dmi.quiesce();
        Ok(())
    }

    // --- Program buffer (Progbuf) Operations ---
    //
    // A QingKe V2 core (the CH32V003 among others) implements **no** abstract
    // memory command: its Debug Module answers cmdtype 2 with cmderr "not
    // supported" and offers register access plus an eight-word program buffer
    // instead (WCH *QingKeV2 Microprocessor Debug Manual* 3.1, `sdi.txt`). Memory
    // and peripherals are reached by having the halted core execute a load or
    // store the debugger placed in progbuf0, with the address and the data
    // bouncing through GPR a0/a1 - the only registers the abstract register
    // command can set and read.
    //
    // One element costs three Debug Module accesses (a0 <- data0 + command,
    // completion wait folded into it, then a1 read back), which is what the
    // legacy C accessor (`riscv32_progbuf_mem_read` in `riscv32.c`) did with its
    // streaming sub-path off.

    /// Read one 32-bit register through the abstract register command.
    fn reg_read(dmi: &mut dyn DmiTransport, regno: u16) -> Result<u32, DmiError> {
        exec_command(dmi, make_reg_access_cmd(regno, false, true, false))?;
        dmi.read(RV_DM_DATA0)
    }

    /// Write one 32-bit register through the abstract register command.
    fn reg_write(dmi: &mut dyn DmiTransport, regno: u16, value: u32) -> Result<(), DmiError> {
        dmi.write(RV_DM_DATA0, value)?;
        exec_command(dmi, make_reg_access_cmd(regno, true, true, false))
    }

    /// Widest element a single load/store covers at `addr` with `len` bytes left.
    ///
    /// A program buffer access is one instruction, so the width follows the
    /// alignment of the address and what is left of the transfer; a transfer that
    /// is not a multiple of the widest width finishes with narrower elements.
    fn progbuf_element_width(addr: u32, len: usize) -> usize {
        if addr % 4 == 0 && len >= 4 {
            4
        } else if addr % 2 == 0 && len >= 2 {
            2
        } else {
            1
        }
    }

    /// The RV32I load that brings `width` bytes from `0(a0)` into `a1`.
    fn progbuf_load(width: usize) -> u32 {
        let load = match width {
            4 => RV_LOAD_WORD | RV_RD_A1,
            2 => RV_LOAD_HALF | RV_RD_A1,
            _ => RV_LOAD_BYTE | RV_RD_A1,
        };
        load | RV_RS1_A0
    }

    /// The RV32I store that puts `width` bytes of `a1` at `0(a0)`.
    fn progbuf_store(width: usize) -> u32 {
        let store = match width {
            4 => RV_STORE_WORD | RV_RS2_A1,
            2 => RV_STORE_HALF | RV_RS2_A1,
            _ => RV_STORE_BYTE | RV_RS2_A1,
        };
        store | RV_RS1_A0
    }

    /// Load the access instruction, then the `ebreak` ending the run (the buffer
    /// is not full, so the core does not append one itself).
    fn progbuf_program(dmi: &mut dyn DmiTransport, program: u32) -> Result<(), DmiError> {
        dmi.write(RV_DM_PROGBUF0, program)?;
        dmi.write(RV_DM_PROGBUF1, RV_EBREAK)
    }

    /// Abstract register number of GPR `gpr` (0..31).
    fn gpr_reg(gpr: u8) -> u16 {
        RV_REG_GPR_BASE + u16::from(gpr)
    }

    /// Read arbitrary memory bytes through the target's program buffer.
    pub(crate) fn progbuf_read(
        dmi: &mut dyn DmiTransport,
        addr: u32,
        dest: &mut [u8],
    ) -> Result<(), TargetError> {
        let saved = Self::progbuf_save_state(dmi);

        let mut programmed = 0usize;
        let mut offset = 0usize;
        while offset < dest.len() {
            let width = Self::progbuf_element_width(addr + offset as u32, dest.len() - offset);
            if programmed != width {
                Self::progbuf_program(dmi, Self::progbuf_load(width))
                    .map_err(|_| TargetError::AccessFault)?;
                programmed = width;
            }
            // a0 <- address, then the load runs as postexec of that command.
            if dmi.write(RV_DM_DATA0, addr + offset as u32).is_err()
                || exec_command(
                    dmi,
                    make_reg_access_cmd(Self::gpr_reg(RV_REG_A0), true, true, true),
                )
                .is_err()
            {
                Self::progbuf_restore_state(dmi, saved);
                return Err(TargetError::AccessFault);
            }
            let value = match Self::reg_read(dmi, Self::gpr_reg(RV_REG_A1)) {
                Ok(value) => value,
                Err(_) => {
                    Self::progbuf_restore_state(dmi, saved);
                    return Err(TargetError::AccessFault);
                }
            };
            dest[offset..offset + width].copy_from_slice(&value.to_le_bytes()[..width]);
            offset += width;
        }

        Self::progbuf_restore_state(dmi, saved);
        Ok(())
    }

    /// Write arbitrary memory bytes through the target's program buffer.
    pub(crate) fn progbuf_write(
        dmi: &mut dyn DmiTransport,
        addr: u32,
        src: &[u8],
    ) -> Result<(), TargetError> {
        let saved = Self::progbuf_save_state(dmi);

        let mut programmed = 0usize;
        let mut offset = 0usize;
        while offset < src.len() {
            let width = Self::progbuf_element_width(addr + offset as u32, src.len() - offset);
            let mut element = [0u8; 4];
            element[..width].copy_from_slice(&src[offset..offset + width]);
            if programmed != width {
                Self::progbuf_program(dmi, Self::progbuf_store(width))
                    .map_err(|_| TargetError::AccessFault)?;
                programmed = width;
            }
            // The address goes in first *without* postexec (or a stale store would
            // run on the a1 write below), then the word plus its postexec store.
            let a0 = Self::gpr_reg(RV_REG_A0);
            if Self::reg_write(dmi, a0, addr + offset as u32).is_err()
                || dmi
                    .write(RV_DM_DATA0, u32::from_le_bytes(element))
                    .is_err()
                || exec_command(dmi, make_reg_access_cmd(Self::gpr_reg(RV_REG_A1), true, true, true))
                    .is_err()
            {
                Self::progbuf_restore_state(dmi, saved);
                return Err(TargetError::AccessFault);
            }
            offset += width;
        }

        Self::progbuf_restore_state(dmi, saved);
        Ok(())
    }

    /// The registers a program buffer run borrows, as this transfer found them.
    fn progbuf_save_state(dmi: &mut dyn DmiTransport) -> [Option<u32>; 3] {
        [
            Self::reg_read(dmi, Self::gpr_reg(RV_REG_A0)).ok(),
            Self::reg_read(dmi, Self::gpr_reg(RV_REG_A1)).ok(),
            Self::reg_read(dmi, RV_CSR_DPC).ok(),
        ]
    }

    /// Put back what [`Self::progbuf_save_state`] found (a failed read is skipped).
    fn progbuf_restore_state(dmi: &mut dyn DmiTransport, saved: [Option<u32>; 3]) {
        let [a0, a1, dpc] = saved;
        if let Some(value) = a0 {
            let _ = Self::reg_write(dmi, Self::gpr_reg(RV_REG_A0), value);
        }
        if let Some(value) = a1 {
            let _ = Self::reg_write(dmi, Self::gpr_reg(RV_REG_A1), value);
        }
        if let Some(value) = dpc {
            let _ = Self::reg_write(dmi, RV_CSR_DPC, value);
        }
    }

    // --- Abstract Autoexec (Streaming) Operations ---

    pub(crate) fn abstract_auto_read(
        dmi: &mut dyn DmiTransport,
        addr: u32,
        dest: &mut [u8],
    ) -> Result<(), TargetError> {
        let word_count = dest.len() / 4;
        if word_count == 0 {
            return Err(TargetError::AccessFault);
        }

        dmi.write(RV_DM_DATA1, addr).map_err(|_| TargetError::AccessFault)?;
        let cmd = (2 << 24) | (0 << 16) | (2 << 20) | (1 << 19); // read, 32-bit, postincrement
        exec_command(dmi, cmd).map_err(|_| TargetError::AccessFault)?;

        if word_count == 1 {
            let val = dmi.read(RV_DM_DATA0).map_err(|_| TargetError::AccessFault)?;
            dest[..4].copy_from_slice(&val.to_le_bytes());
            dmi.quiesce();
            return Ok(());
        }

        dmi.write(RV_DM_ABSTRACTAUTO, 1).map_err(|_| TargetError::AccessFault)?;
        for i in 0..(word_count - 1) {
            let val = match dmi.read(RV_DM_DATA0) {
                Ok(v) => v,
                Err(_) => {
                    let _ = dmi.write(RV_DM_ABSTRACTAUTO, 0);
                    return Err(TargetError::AccessFault);
                }
            };
            dest[i * 4..(i + 1) * 4].copy_from_slice(&val.to_le_bytes());
        }

        dmi.write(RV_DM_ABSTRACTAUTO, 0).map_err(|_| TargetError::AccessFault)?;
        let last_val = dmi.read(RV_DM_DATA0).map_err(|_| TargetError::AccessFault)?;
        let last_idx = word_count - 1;
        dest[last_idx * 4..(last_idx + 1) * 4].copy_from_slice(&last_val.to_le_bytes());

        dmi.quiesce();
        Ok(())
    }

    pub(crate) fn abstract_auto_write(
        dmi: &mut dyn DmiTransport,
        addr: u32,
        src: &[u8],
    ) -> Result<(), TargetError> {
        let word_count = src.len() / 4;
        if word_count == 0 {
            return Err(TargetError::AccessFault);
        }

        dmi.write(RV_DM_DATA1, addr).map_err(|_| TargetError::AccessFault)?;
        let first_val = u32::from_le_bytes(src[..4].try_into().unwrap());
        dmi.write(RV_DM_DATA0, first_val).map_err(|_| TargetError::AccessFault)?;

        let cmd = (2 << 24) | (1 << 16) | (2 << 20) | (1 << 19); // write, 32-bit, postincrement
        exec_command(dmi, cmd).map_err(|_| TargetError::AccessFault)?;

        dmi.write(RV_DM_ABSTRACTAUTO, 1).map_err(|_| TargetError::AccessFault)?;
        for i in 1..word_count {
            let val = u32::from_le_bytes(src[i * 4..(i + 1) * 4].try_into().unwrap());
            if dmi.write(RV_DM_DATA0, val).is_err() {
                let _ = dmi.write(RV_DM_ABSTRACTAUTO, 0);
                return Err(TargetError::AccessFault);
            }
        }

        wait_command_complete(dmi).map_err(|_| TargetError::AccessFault)?;
        dmi.write(RV_DM_ABSTRACTAUTO, 0).map_err(|_| TargetError::AccessFault)?;
        wait_command_complete(dmi).map_err(|_| TargetError::AccessFault)?;
        dmi.quiesce();
        Ok(())
    }
}
