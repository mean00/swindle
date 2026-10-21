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
use super::dmi::{DmiError, DmiTransport};
use super::hart::{MemoryAccessMethod, RiscvHart};
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

/// Memory access dispatcher for RISC-V targets.
pub struct RiscvMem;

impl RiscvMem {
    /// Read arbitrary memory bytes from `addr` into `dest`.
    pub fn read(
        hart: &mut RiscvHart,
        addr: u32,
        dest: &mut [u8],
    ) -> Result<(), TargetError> {
        if dest.is_empty() {
            return Ok(());
        }

        // Bounded retry loop for SDI read confirmation (patch 102 & 103)
        let retry_count = if hart.dmi.needs_read_confirm() { 4 } else { 1 };
        for _ in 0..retry_count {
            if Self::read_attempt(hart, addr, dest).is_ok() {
                return Ok(());
            }
        }
        Err(TargetError::AccessFault)
    }

    fn read_attempt(
        hart: &mut RiscvHart,
        addr: u32,
        dest: &mut [u8],
    ) -> Result<(), TargetError> {
        match hart.active_method {
            MemoryAccessMethod::Sysbus => Self::sysbus_read(&mut hart.dmi, addr, dest),
            MemoryAccessMethod::AbstAuto => {
                if dest.len() <= 16 {
                    Self::abstract_read(&mut hart.dmi, addr, dest)
                } else {
                    let mut cur_addr = addr;
                    let mut remaining = dest;
                    let unaligned = (cur_addr & 3) as usize;
                    if unaligned != 0 {
                        let leading = core::cmp::min(4 - unaligned, remaining.len());
                        Self::abstract_read(&mut hart.dmi, cur_addr, &mut remaining[..leading])?;
                        cur_addr += leading as u32;
                        remaining = &mut remaining[leading..];
                    }
                    let aligned_len = remaining.len() & !3;
                    if aligned_len >= 16 {
                        Self::abstract_auto_read(&mut hart.dmi, cur_addr, &mut remaining[..aligned_len])?;
                        cur_addr += aligned_len as u32;
                        remaining = &mut remaining[aligned_len..];
                    }
                    if !remaining.is_empty() {
                        Self::abstract_read(&mut hart.dmi, cur_addr, remaining)?;
                    }
                    Ok(())
                }
            }
            MemoryAccessMethod::Abstract | MemoryAccessMethod::Progbuf => {
                Self::abstract_read(&mut hart.dmi, addr, dest)
            }
        }
    }

    /// Write arbitrary memory bytes from `src` to `addr`.
    pub fn write(
        hart: &mut RiscvHart,
        addr: u32,
        src: &[u8],
    ) -> Result<(), TargetError> {
        if src.is_empty() {
            return Ok(());
        }
        match hart.active_method {
            MemoryAccessMethod::Sysbus => Self::sysbus_write(&mut hart.dmi, addr, src),
            MemoryAccessMethod::AbstAuto => {
                if src.len() <= 16 {
                    Self::abstract_write(&mut hart.dmi, addr, src)
                } else {
                    let mut cur_addr = addr;
                    let mut remaining = src;
                    let unaligned = (cur_addr & 3) as usize;
                    if unaligned != 0 {
                        let leading = core::cmp::min(4 - unaligned, remaining.len());
                        Self::abstract_write(&mut hart.dmi, cur_addr, &remaining[..leading])?;
                        cur_addr += leading as u32;
                        remaining = &remaining[leading..];
                    }
                    let aligned_len = remaining.len() & !3;
                    if aligned_len >= 16 {
                        Self::abstract_auto_write(&mut hart.dmi, cur_addr, &remaining[..aligned_len])?;
                        cur_addr += aligned_len as u32;
                        remaining = &remaining[aligned_len..];
                    }
                    if !remaining.is_empty() {
                        Self::abstract_write(&mut hart.dmi, cur_addr, remaining)?;
                    }
                    Ok(())
                }
            }
            MemoryAccessMethod::Abstract | MemoryAccessMethod::Progbuf => {
                Self::abstract_write(&mut hart.dmi, addr, src)
            }
        }
    }

    /// Non-halting read via Sysbus (returns true if performed).
    pub fn read_nostop(
        hart: &mut RiscvHart,
        addr: u32,
        dest: &mut [u8],
    ) -> Result<bool, TargetError> {
        if hart.has_sysbus {
            Self::read(hart, addr, dest).map(|_| true)
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
            Self::write(hart, addr, src).map(|_| true)
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
