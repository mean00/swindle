/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2023-2024 1BitSquared <info@1bitsquared.com>
 * Written by Rachel Mant <git@dragonmux.network>
 * All rights reserved.
 *
 * Rust conversion of blackmagic/src/target/riscv_debug.c and
 * blackmagic/src/target/riscv32.c, made for the Swindle firmware. This file is
 * a translation of those C sources into Rust and stays under the same license as
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

//! RISC-V Hart representation, register file, and run control (RV32E, RV32I, RV32IF).

use super::dm::*;
use super::dm_regs::*;
use super::dmi::{CDmiTransport, DmiError, DmiTransport};
use super::rv_csr::*;
use super::triggers::TriggerUnit;
use crate::reset_pin;
use crate::target::driver::{HaltReason, TargetError};

/// How long `reset()` waits for the Debug Module to report that the reset reached the hart.
///
/// The legacy C driver waited 500 ms for the same condition in its part-specific reset.
const RESET_ALL_RESET_TIMEOUT_MS: u32 = 500;

/// How long the nRST pulse driven by [`RiscvHart::reset_via_pin`] is held, in ms.
///
/// 1 ms, the width the Cortex-M sequence uses (`reset_pin::pulse(1)`): far longer than any
/// reset input filter, far shorter than a user would notice in a `mon reset`.
const RESET_PIN_PULSE_MS: u32 = 1;

/// How long [`RiscvHart::halt_request`] and [`RiscvHart::halt_resume`] wait for the Debug
/// Module to report the state change they asked for, in ms.
///
/// Both loops used to be a flat 1000 polls with no delay between them: on a fast transport
/// that gave up after a few milliseconds, on a slow one the same loop sat inside the DMI
/// layer for minutes - and a WCH DM that never acknowledges the request turned a
/// `mon riscv_benchmark` into a wedged GDB CDC. Bounding the wait in *time* makes the
/// failure reportable, the way [`RiscvHart::wait_all_reset`] already was.
const RUN_CONTROL_TIMEOUT_MS: u32 = 250;

/// Settle time after nRST is released, before the first DMI access.
///
/// The part needs its clocks up before the Debug Module answers again; the Cortex-M sequence
/// waits the same 10 ms after its pulse. The re-activation loop in [`RiscvHart::reset_via_pin`]
/// covers a part that is slower still.
const RESET_PIN_SETTLE_MS: u32 = 10;

/// Supported RISC-V 32-bit core architectures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HartIsa {
    /// RV32E: 16 GPRs + PC (68 bytes register file).
    Rv32e,
    /// RV32I: 32 GPRs + PC (132 bytes register file).
    Rv32i,
    /// RV32I + Single-precision FPU: 32 GPRs + PC + 32 FPRs + FPU CSRs.
    Rv32iSingleFpu,
}

impl HartIsa {
    /// Number of general purpose registers (16 for RV32E, 32 for RV32I/IF).
    pub fn gpr_count(self) -> usize {
        match self {
            HartIsa::Rv32e => 16,
            HartIsa::Rv32i | HartIsa::Rv32iSingleFpu => 32,
        }
    }

    /// GDB register index of the program counter (16 for RV32E, 32 for RV32I/IF).
    pub fn pc_reg_num(self) -> u32 {
        self.gpr_count() as u32
    }

    /// Total size in bytes of bulk general registers (`regs_read`/`regs_write`).
    pub fn regs_size(self) -> usize {
        (self.gpr_count() + 1) * 4
    }

    /// True if single-precision floating point extension ('F') is supported.
    pub fn has_single_fpu(self) -> bool {
        matches!(self, HartIsa::Rv32iSingleFpu)
    }
}

/// Memory access methods available for RISC-V debug target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAccessMethod {
    Abstract,
    AbstAuto,
    Progbuf,
    Sysbus,
}

impl MemoryAccessMethod {
    /// Wire value of the `method` parameter of `RV.MEM_READ` / `RV.MEM_WRITE`.
    ///
    /// The hosted build sends the method it selected so the probe runs the same
    /// access sequence instead of guessing one. Both ends compile this file, so
    /// the numbering cannot drift; the values are still part of the wire format.
    pub const fn wire_id(self) -> u8 {
        match self {
            Self::Abstract => 0,
            Self::AbstAuto => 1,
            Self::Progbuf => 2,
            Self::Sysbus => 3,
        }
    }

    /// Decode a [`Self::wire_id`]; `None` for a value this build does not know
    /// (a newer host talking to an older probe).
    pub const fn from_wire_id(id: u32) -> Option<Self> {
        match id {
            0 => Some(Self::Abstract),
            1 => Some(Self::AbstAuto),
            2 => Some(Self::Progbuf),
            3 => Some(Self::Sysbus),
            _ => None,
        }
    }
}

/// RISC-V Hart state and controller.
pub struct RiscvHart {
    pub dmi: CDmiTransport,
    pub isa: HartIsa,
    pub hart_idx: u32,
    pub hartsel: u32,
    pub vendor_id: u32,
    pub arch_id: u32,
    pub impl_id: u32,
    pub hart_id: u32,
    pub progbuf_size: u8,
    pub has_sysbus: bool,
    pub has_abstauto: bool,
    pub active_method: MemoryAccessMethod,
    /// Hardware breakpoint / watchpoint capability of this hart's trigger unit.
    ///
    /// Filled by [`super::triggers::HwTriggers::resolve`] during the driver probe;
    /// [`TriggerUnit::NONE`] until then (or on parts that were never scanned).
    pub triggers: TriggerUnit,
    /// Cached halt reason while the core remains continuously halted.
    pub last_halt_reason: Option<HaltReason>,
    /// Last configured single-step mode on DCSR (to avoid redundant DCSR writes during resume)
    pub last_step_mode: Option<bool>,
}

impl RiscvHart {
    /// Create a new Hart descriptor and discover core capabilities.
    pub fn probe(mut dmi: CDmiTransport, hart_idx: u32) -> Result<Self, DmiError> {
        let hartsel = hartsel_mask(hart_idx);

        // Select the hart and ensure active
        dmi.write(RV_DM_CONTROL, hartsel | RV_DM_CTRL_ACTIVE)?;
        let status = dmi.read(RV_DM_STATUS)?;
        if (status & (RV_DM_STAT_ANY_NONEXIST | RV_DM_STAT_ANY_UNAVAIL)) != 0 {
            return Err(DmiError::NotSupported);
        }

        // Clear any residual cmderr in abstractcs
        let abstcs_init = dmi.read(RV_DM_ABSTRACTCS).unwrap_or(0);
        if (abstcs_init & RV_DM_ABSTCS_CMDERR_MASK) != 0 {
            let _ = dmi.write(RV_DM_ABSTRACTCS, abstcs_init);
        }

        // Discover progbuf size and data count
        let abstcs = dmi.read(RV_DM_ABSTRACTCS)?;
        let progbuf_size = ((abstcs & RV_DM_ABSTCS_PROGBUF_SIZE_MASK) >> RV_DM_ABSTCS_PROGBUF_SIZE_SHIFT) as u8;

        // Check sysbus support
        let sbcs = dmi.read(RV_DM_SBCS).unwrap_or(0);
        let has_sysbus = (sbcs & 0xfe0) != 0; // sbasize > 0 (bits 11:5)
        if has_sysbus {
            let _ = dmi.write(RV_DM_SBCS, 0x00407000); // clear sbbusyerror and sberror
        }

        // Check abstractauto support
        let has_abstauto = if dmi.write(RV_DM_ABSTRACTAUTO, 1).is_ok() {
            let auto = dmi.read(RV_DM_ABSTRACTAUTO).unwrap_or(0);
            let _ = dmi.write(RV_DM_ABSTRACTAUTO, 0);
            (auto & 1) != 0
        } else {
            false
        };

        let mut hart = Self {
            dmi,
            isa: HartIsa::Rv32i, // Temporary default
            hart_idx,
            hartsel,
            vendor_id: 0,
            arch_id: 0,
            impl_id: 0,
            hart_id: 0,
            progbuf_size,
            has_sysbus,
            has_abstauto,
            active_method: MemoryAccessMethod::Abstract,
            triggers: TriggerUnit::NONE,
            last_halt_reason: None,
            last_step_mode: None,
        };

        // Halt hart briefly to read IDs and MISA safely
        hart.halt_request()?;

        // Discover ISA from MISA
        let misa = hart.csr_read(RV_CSR_MISA).unwrap_or(0);
        if (misa & RV_MISA_EXT_E) != 0 {
            hart.isa = HartIsa::Rv32e;
        } else if (misa & RV_MISA_EXT_F) != 0 {
            hart.isa = HartIsa::Rv32iSingleFpu;
        } else {
            hart.isa = HartIsa::Rv32i;
        }

        // Read identifiers
        hart.vendor_id = hart.csr_read(RV_CSR_MVENDORID).unwrap_or(0);
        hart.arch_id = hart.csr_read(RV_CSR_MARCHID).unwrap_or(0);
        hart.impl_id = hart.csr_read(RV_CSR_MIMPID).unwrap_or(0);
        hart.hart_id = hart.csr_read(RV_CSR_MHARTID).unwrap_or(0);

        // Which accessors this Debug Module actually has, and which of them is the
        // fastest, has to be settled here - before the first memory read. The
        // per-family driver probe reads its signature out of the ESIG window first
        // thing, and a part whose DM has no abstract memory command (the QingKe V2
        // core in the CH32V003 answers it with cmderr "not supported") can only be
        // identified through its program buffer. Detection *is* this benchmark:
        // no register says which commands a DM implements.
        super::benchmark::auto_select_fastest_method(&mut hart);

        Ok(hart)
    }

    /// Access the underlying DMI transport.
    pub fn dmi(&self) -> &CDmiTransport {
        &self.dmi
    }

    /// Access the underlying DMI transport mutably.
    pub fn dmi_mut(&mut self) -> &mut CDmiTransport {
        &mut self.dmi
    }

    /// Read a 32-bit CSR using abstract register access command.
    pub fn csr_read(&mut self, csr: u16) -> Result<u32, DmiError> {
        let cmd = make_reg_access_cmd(csr, false, true, false);
        exec_command(&mut self.dmi, cmd)?;
        self.dmi.read(RV_DM_DATA0)
    }

    /// Write a 32-bit CSR using abstract register access command.
    pub fn csr_write(&mut self, csr: u16, val: u32) -> Result<(), DmiError> {
        if csr == RV_CSR_DCSR {
            self.last_step_mode = None;
        }
        self.dmi.write(RV_DM_DATA0, val)?;
        let cmd = make_reg_access_cmd(csr, true, true, false);
        exec_command(&mut self.dmi, cmd)
    }

    /// Read a single GPR (0..31).
    pub fn gpr_read(&mut self, gpr: u8) -> Result<u32, DmiError> {
        self.csr_read(RV_REG_GPR_BASE + gpr as u16)
    }

    /// Write a single GPR (0..31).
    pub fn gpr_write(&mut self, gpr: u8, val: u32) -> Result<(), DmiError> {
        self.csr_write(RV_REG_GPR_BASE + gpr as u16, val)
    }

    /// Read program counter (DPC).
    pub fn pc_read(&mut self) -> Result<u32, DmiError> {
        self.csr_read(RV_CSR_DPC)
    }

    /// Write program counter (DPC).
    pub fn pc_write(&mut self, val: u32) -> Result<(), DmiError> {
        self.csr_write(RV_CSR_DPC, val)
    }

    /// Read a single register by GDB register number into `buf`.
    pub fn reg_read(&mut self, reg: u32, buf: &mut [u8]) -> Result<usize, TargetError> {
        if buf.len() < 4 {
            return Err(TargetError::InternalError);
        }
        let gprs_count = self.isa.gpr_count() as u32;

        let val = if reg < gprs_count {
            self.gpr_read(reg as u8).map_err(|_| TargetError::AccessFault)?
        } else if reg == gprs_count {
            self.pc_read().map_err(|_| TargetError::AccessFault)?
        } else if self.isa.has_single_fpu() && (RV_FPU_GDB_OFFSET..=64).contains(&reg) {
            // f0..f31 single-precision FPRs (0x1020..0x103f)
            let fpr = (reg - RV_FPU_GDB_OFFSET) as u16;
            self.csr_read(RV_REG_FPR_BASE + fpr).map_err(|_| TargetError::AccessFault)?
        } else if self.isa.has_single_fpu() && (RV_FPU_GDB_CSR_OFFSET..=68).contains(&reg) {
            // fflags (0x001), frm (0x002), fcsr (0x003)
            let csr = (reg - RV_FPU_GDB_CSR_OFFSET + 1) as u16;
            self.csr_read(csr).map_err(|_| TargetError::AccessFault)?
        } else if reg >= RV_CSR_GDB_OFFSET {
            let csr = (reg - RV_CSR_GDB_OFFSET) as u16;
            self.csr_read(csr).map_err(|_| TargetError::AccessFault)?
        } else {
            return Err(TargetError::Unsupported);
        };

        buf[..4].copy_from_slice(&val.to_le_bytes());
        Ok(4)
    }

    /// Write a single register by GDB register number from `data`.
    pub fn reg_write(&mut self, reg: u32, data: &[u8]) -> Result<usize, TargetError> {
        if data.len() < 4 {
            return Err(TargetError::InternalError);
        }
        let val = u32::from_le_bytes(data[..4].try_into().unwrap());
        let gprs_count = self.isa.gpr_count() as u32;

        if reg < gprs_count {
            self.gpr_write(reg as u8, val).map_err(|_| TargetError::AccessFault)?;
        } else if reg == gprs_count {
            self.pc_write(val).map_err(|_| TargetError::AccessFault)?;
        } else if self.isa.has_single_fpu() && (RV_FPU_GDB_OFFSET..=64).contains(&reg) {
            let fpr = (reg - RV_FPU_GDB_OFFSET) as u16;
            self.csr_write(RV_REG_FPR_BASE + fpr, val).map_err(|_| TargetError::AccessFault)?;
        } else if self.isa.has_single_fpu() && (RV_FPU_GDB_CSR_OFFSET..=68).contains(&reg) {
            let csr = (reg - RV_FPU_GDB_CSR_OFFSET + 1) as u16;
            self.csr_write(csr, val).map_err(|_| TargetError::AccessFault)?;
        } else if reg >= RV_CSR_GDB_OFFSET {
            let csr = (reg - RV_CSR_GDB_OFFSET) as u16;
            self.csr_write(csr, val).map_err(|_| TargetError::AccessFault)?;
        } else {
            return Err(TargetError::Unsupported);
        }

        Ok(4)
    }

    /// Read all general registers (GPRs + PC) into `buf`.
    pub fn regs_read(&mut self, buf: &mut [u8]) -> Result<(), TargetError> {
        let count = self.isa.gpr_count();
        let total_size = (count + 1) * 4;
        if buf.len() < total_size {
            return Err(TargetError::InternalError);
        }

        // x0 (zero) is hardwired to 0 in RISC-V
        buf[0..4].fill(0);
        for i in 1..count {
            let val = self.gpr_read(i as u8).map_err(|_| TargetError::AccessFault)?;
            buf[i * 4..(i + 1) * 4].copy_from_slice(&val.to_le_bytes());
        }
        let pc = self.pc_read().map_err(|_| TargetError::AccessFault)?;
        buf[count * 4..(count + 1) * 4].copy_from_slice(&pc.to_le_bytes());

        Ok(())
    }

    /// Write all general registers (GPRs + PC) from `data`.
    pub fn regs_write(&mut self, data: &[u8]) -> Result<(), TargetError> {
        let count = self.isa.gpr_count();
        let total_size = (count + 1) * 4;
        if data.len() < total_size {
            return Err(TargetError::InternalError);
        }

        // Skip x0 (zero), which is hardwired to 0 in RISC-V
        for i in 1..count {
            let val = u32::from_le_bytes(data[i * 4..(i + 1) * 4].try_into().unwrap());
            self.gpr_write(i as u8, val).map_err(|_| TargetError::AccessFault)?;
        }
        let pc = u32::from_le_bytes(data[count * 4..(count + 1) * 4].try_into().unwrap());
        self.pc_write(pc).map_err(|_| TargetError::AccessFault)?;

        Ok(())
    }

    /// Request halt of target core.
    ///
    /// Bounded in time ([`RUN_CONTROL_TIMEOUT_MS`]) so a Debug Module that never reports
    /// `allhalted` costs a fraction of a second and a log line, not a stuck task.
    pub fn halt_request(&mut self) -> Result<(), DmiError> {
        self.dmi
            .write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE | RV_DM_CTRL_HALT_REQ)?;
        let mut last_status = 0u32;
        for i in 0..(RUN_CONTROL_TIMEOUT_MS + 8) {
            let status = self.dmi.read(RV_DM_STATUS)?;
            last_status = status;
            if (status & RV_DM_STAT_ALL_HALTED) != 0 {
                // Clear haltreq once halted, matching C Blackmagic line 1337 and sdi.txt Table 3-16
                self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE)?;
                self.last_halt_reason = None;
                return Ok(());
            }
            if i >= 8 {
                crate::target::delay_ms(1);
            }
        }
        let _ = self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE);
        self.last_halt_reason = None;
        crate::gdb_println!("halt: no allhalted within timeout, dmstatus=", Hex(last_status));
        Err(DmiError::Timeout)
    }

    /// Resume core execution, optionally single-stepping.
    ///
    /// Follows RISC-V Debug Spec 0.13 §3.12.1 resume protocol without illegal abstract
    /// command access or stale cause checks during resume transition.
    pub fn halt_resume(&mut self, step: bool) -> Result<(), DmiError> {
        self.last_halt_reason = None;

        // Update dcsr only if step mode has changed or not yet initialized
        if self.last_step_mode != Some(step) {
            let mut dcsr = self.csr_read(RV_CSR_DCSR).unwrap_or(0);
            if step {
                dcsr |= RV_DCSR_STEP;
            } else {
                dcsr &= !RV_DCSR_STEP;
            }
            // Always ensure ebreakm is set (patch 104)
            dcsr |= RV_DCSR_EBREAKM;
            let _ = self.csr_write(RV_CSR_DCSR, dcsr);
            self.last_step_mode = Some(step);
        }

        // Send resume request
        self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE | RV_DM_CTRL_HART_RESUME_REQ)?;
        let mut last_status = 0u32;
        for i in 0..(RUN_CONTROL_TIMEOUT_MS + 8) {
            let status = self.dmi.read(RV_DM_STATUS)?;
            last_status = status;
            if (status & (RV_DM_STAT_ALL_RUNNING | RV_DM_STAT_ALL_RESUMEACK)) != 0
                || (status & RV_DM_STAT_ALL_HALTED) == 0
            {
                // Target acknowledged resume request / is running
                let _ = self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE);
                return Ok(());
            }
            if i >= 8 {
                crate::target::delay_ms(1);
            }
        }
        let _ = self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE);
        crate::gdb_println!("resume: no allrunning within timeout, dmstatus=", Hex(last_status));
        Err(DmiError::Timeout)
    }

    /// Poll core execution state.
    pub fn halt_poll(&mut self) -> (HaltReason, u32) {
        let status = match self.dmi.read(RV_DM_STATUS) {
            Ok(s) => s,
            Err(_) => {
                self.last_halt_reason = None;
                return (HaltReason::Error, 0);
            }
        };

        if (status & RV_DM_STAT_ALL_HALTED) != 0 {
            if let Some(reason) = self.last_halt_reason {
                return (reason, 0);
            }
            let dcsr = self.csr_read(RV_CSR_DCSR).unwrap_or(0);
            let cause = (dcsr & RV_DCSR_CAUSE_MASK) >> RV_DCSR_CAUSE_SHIFT;
            let reason = match cause {
                1 => HaltReason::Breakpoint, // ebreak
                2 => HaltReason::Watchpoint, // trigger
                3 => HaltReason::Request,    // haltreq
                4 => HaltReason::Stepping,   // step
                _ => HaltReason::Unknown,
            };
            self.last_halt_reason = Some(reason);
            (reason, 0)
        } else if (status & RV_DM_STAT_ALL_RUNNING) != 0 {
            self.last_halt_reason = None;
            (HaltReason::Running, 0)
        } else {
            self.last_halt_reason = None;
            (HaltReason::Unknown, 0)
        }
    }

    /// Reset the target through the Debug Module's `ndmreset`, with halt-request asserted.
    pub fn reset(&mut self) -> Result<(), DmiError> {
        self.last_halt_reason = None;
        self.last_step_mode = None;

        // Clear what `allhavereset` may still be remembering from an earlier reset
        self.dmi.write(
            RV_DM_CONTROL,
            self.hartsel | RV_DM_CTRL_ACTIVE | RV_DM_CTRL_HALT_REQ | RV_DM_CTRL_HART_ACK_RESET,
        )?;

        // Assert haltreq and ndmreset together
        self.dmi.write(
            RV_DM_CONTROL,
            self.hartsel | RV_DM_CTRL_ACTIVE | RV_DM_CTRL_HALT_REQ | RV_DM_CTRL_NDM_RESET,
        )?;

        // Release ndmreset while maintaining haltreq
        self.dmi.write(
            RV_DM_CONTROL,
            self.hartsel | RV_DM_CTRL_ACTIVE | RV_DM_CTRL_HALT_REQ,
        )?;

        // Wait until Debug Module reports reset reached the hart
        let already_halted = self.wait_all_reset();

        // Acknowledge the reset while holding haltreq
        self.dmi.write(
            RV_DM_CONTROL,
            self.hartsel | RV_DM_CTRL_ACTIVE | RV_DM_CTRL_HALT_REQ | RV_DM_CTRL_HART_ACK_RESET,
        )?;

        // Ensure halted (if wait_all_reset already observed allhalted, just clear haltreq)
        if already_halted {
            self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE)?;
        } else {
            self.halt_request()?;
        }

        self.dmi.invalidate_caches();

        // Set ebreakm in dcsr so software breakpoints trigger debug halt
        let mut dcsr = self.csr_read(RV_CSR_DCSR).unwrap_or(0);
        dcsr |= RV_DCSR_EBREAKM;
        let _ = self.csr_write(RV_CSR_DCSR, dcsr);

        Ok(())
    }

    /// Give the part a hardware reset through the probe's nRST line, then bring the Debug Module
    /// back up, ready for [`Self::reset`].
    pub fn pulse_nrst(&mut self) -> Result<(), DmiError> {
        self.last_halt_reason = None;

        reset_pin::pulse(RESET_PIN_PULSE_MS);
        crate::target::delay_ms(RESET_PIN_SETTLE_MS);

        let control = self.hartsel | RV_DM_CTRL_ACTIVE | RV_DM_CTRL_HALT_REQ;
        for _ in 0..RESET_ALL_RESET_TIMEOUT_MS {
            if self.dmi.write(RV_DM_CONTROL, control).is_ok()
                && matches!(self.dmi.read(RV_DM_CONTROL), Ok(value) if value & RV_DM_CTRL_ACTIVE != 0)
            {
                return Ok(());
            }
            crate::target::delay_ms(1);
        }
        Err(DmiError::Timeout)
    }

    /// Wait (bounded) for `dmstatus.allhavereset`, returning true if `allhalted` is also set.
    fn wait_all_reset(&mut self) -> bool {
        // Bounded wait for dmstatus.allhavereset (100 ms max, with 8 fast polls first)
        for i in 0..108 {
            match self.dmi.read(RV_DM_STATUS) {
                Ok(status) if (status & RV_DM_STAT_ALL_HAVERESET) != 0 => {
                    return (status & RV_DM_STAT_ALL_HALTED) != 0;
                }
                Err(_) => return false,
                Ok(_) => {
                    if i >= 8 {
                        crate::target::delay_ms(1);
                    }
                }
            }
        }
        false
    }

    // --- Direct DM / DMI access ---
    #[inline]
    pub fn dm_read(&mut self, addr: u8) -> Result<u32, DmiError> {
        self.dmi.read(addr)
    }

    #[inline]
    pub fn dm_write(&mut self, addr: u8, val: u32) -> Result<(), DmiError> {
        self.dmi.write(addr, val)
    }

    // --- Direct Memory Access (Native Rust API) ---
    //
    // Hosted builds hand the block to the probe (`RV.MEM_READ` / `RV.MEM_WRITE`):
    // there every DM access is an RPC round trip, so running the access sequence
    // per word is what capped a `load` at 2 KB/s. The probe runs the very same
    // sequence from its own DMI (see `hosted/bulk_mem.rs`).
    //
    // `swindle_hosted --raw-dmi` turns that around again: the sequence runs here
    // and each DM op is its own round trip, which is the pre-bulk behaviour.
    pub fn mem_read(&mut self, addr: u32, dest: &mut [u8]) -> Result<(), TargetError> {
        #[cfg(feature = "hosted")]
        if !crate::hosted::bulk_mem::raw_dmi() {
            return crate::hosted::bulk_mem::read(self.active_method, addr, dest);
        }
        super::mem::RiscvMem::read(self, addr, dest).map_err(|_| TargetError::AccessFault)
    }

    pub fn mem_write(&mut self, addr: u32, src: &[u8]) -> Result<(), TargetError> {
        #[cfg(feature = "hosted")]
        if !crate::hosted::bulk_mem::raw_dmi() {
            return crate::hosted::bulk_mem::write(self.active_method, addr, src);
        }
        super::mem::RiscvMem::write(self, addr, src).map_err(|_| TargetError::AccessFault)
    }

    pub fn mem_read_nostop(&mut self, addr: u32, dest: &mut [u8]) -> Result<bool, TargetError> {
        super::mem::RiscvMem::read_nostop(self, addr, dest).map_err(|_| TargetError::AccessFault)
    }

    pub fn mem_write_nostop(&mut self, addr: u32, src: &[u8]) -> Result<bool, TargetError> {
        super::mem::RiscvMem::write_nostop(self, addr, src).map_err(|_| TargetError::AccessFault)
    }

    pub fn mem_read32(&mut self, addr: u32) -> Result<u32, TargetError> {
        let mut buf = [0u8; 4];
        self.mem_read(addr, &mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    pub fn mem_read16(&mut self, addr: u32) -> Result<u16, TargetError> {
        let mut buf = [0u8; 2];
        self.mem_read(addr, &mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    pub fn mem_write32(&mut self, addr: u32, val: u32) -> Result<(), TargetError> {
        self.mem_write(addr, &val.to_le_bytes())
    }

    pub fn mem_write16(&mut self, addr: u32, val: u16) -> Result<(), TargetError> {
        self.mem_write(addr, &val.to_le_bytes())
    }

    // --- Stub Execution (Native Rust API) ---

    /// Run the stub at `loadaddr` with `p1..p4` in `a0..a3`.
    ///
    /// `true` means the stub published the common exit code (`STUB_EXIT_OK`, i.e. `a0 = 1`)
    /// once it was done; see [`crate::stub_exit`].
    pub fn run_stub(&mut self, loadaddr: u32, p1: u32, p2: u32, p3: u32, p4: u32) -> bool {
        super::stub::run_stub(self, loadaddr, p1, p2, p3, p4)
    }

    /// Run the stub with an explicit stack pointer, reporting whether it published
    /// `STUB_EXIT_OK` in `a0`.
    pub fn run_stub_with_stack(&mut self, loadaddr: u32, stack: u32, p1: u32, p2: u32, p3: u32, p4: u32) -> bool {
        super::stub::run_stub_with_stack(self, loadaddr, stack, p1, p2, p3, p4)
    }

    /// Re-activate the debug module (`dmcontrol.dmactive`) and confirm it took.
    ///
    /// The flash-mode entry resets the target (`target_enter_flash_mode()`), and that
    /// reset leaves the DM inactive. An inactive DM still accepts register writes and
    /// still reports plausible status, it just does not *steer the core* - so every
    /// write the stub runner makes would be accepted and ignored.
    pub fn dm_activate(&mut self) -> bool {
        for _ in 0..3 {
            let _ = self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE);
            if let Ok(control) = self.dmi.read(RV_DM_CONTROL)
                && (control & RV_DM_CTRL_ACTIVE) != 0
            {
                return true;
            }
        }
        false
    }

    /// [`Self::run_stub_with_stack`] with an explicit bound on how long the polling may
    /// take, in milliseconds; `None` means the default of one second (see
    /// [`super::stub::run_stub_with_stack_timeout`]).
    pub fn run_stub_with_stack_timeout(
        &mut self,
        loadaddr: u32,
        stack: u32,
        p1: u32,
        p2: u32,
        p3: u32,
        p4: u32,
        timeout: Option<u32>,
    ) -> bool {
        super::stub::run_stub_with_stack_timeout(self, loadaddr, stack, p1, p2, p3, p4, timeout)
    }

    /// [`Self::run_stub_with_stack_timeout`] plus a silence of `hold_ms` milliseconds
    /// between the resume and the first `dmstatus` read (see
    /// [`super::stub::run_stub_with_stack_hold`]).
    #[allow(clippy::too_many_arguments)]
    pub fn run_stub_with_stack_hold(
        &mut self,
        loadaddr: u32,
        stack: u32,
        p1: u32,
        p2: u32,
        p3: u32,
        p4: u32,
        timeout: Option<u32>,
        hold_ms: u32,
    ) -> bool {
        super::stub::run_stub_with_stack_hold(self, loadaddr, stack, p1, p2, p3, p4, timeout, hold_ms)
    }

    /// Like [`Self::run_stub_with_stack`] but returns the result the stub left in `a2`
    /// (`None` when the stub did not report `STUB_EXIT_OK`).
    pub fn run_stub_with_stack_result(
        &mut self,
        loadaddr: u32,
        stack: u32,
        p1: u32,
        p2: u32,
        p3: u32,
        p4: u32,
    ) -> Option<u32> {
        super::stub::run_stub_with_stack_result(self, loadaddr, stack, p1, p2, p3, p4)
    }

    /// Alias of [`Self::run_stub_with_stack_result`].
    pub fn run_stub_with_stack_ret(
        &mut self,
        loadaddr: u32,
        stack: u32,
        p1: u32,
        p2: u32,
        p3: u32,
        p4: u32,
    ) -> Option<u32> {
        super::stub::run_stub_with_stack_ret(self, loadaddr, stack, p1, p2, p3, p4)
    }
}

#[cfg(test)]
mod tests {
    use super::MemoryAccessMethod;

    /// The `method` byte of `RV.MEM_READ` / `RV.MEM_WRITE` is a wire value: it has
    /// to survive the round trip and stay stable (a probe and a host from
    /// different builds may meet).
    #[test]
    fn memory_access_method_wire_ids_round_trip() {
        for method in [
            MemoryAccessMethod::Abstract,
            MemoryAccessMethod::AbstAuto,
            MemoryAccessMethod::Progbuf,
            MemoryAccessMethod::Sysbus,
        ] {
            assert_eq!(
                MemoryAccessMethod::from_wire_id(method.wire_id() as u32),
                Some(method)
            );
        }
        // The numbering is part of the wire format, not an implementation detail.
        assert_eq!(MemoryAccessMethod::Abstract.wire_id(), 0);
        assert_eq!(MemoryAccessMethod::AbstAuto.wire_id(), 1);
        assert_eq!(MemoryAccessMethod::Progbuf.wire_id(), 2);
        assert_eq!(MemoryAccessMethod::Sysbus.wire_id(), 3);
        // An unknown id is rejected instead of silently mapping onto one of ours.
        assert_eq!(MemoryAccessMethod::from_wire_id(4), None);
    }
}
