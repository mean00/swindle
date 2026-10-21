/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2023-2024 1BitSquared <info@1bitsquared.com>
 * Written by Rachel Mant <git@dragonmux.network>
 * All rights reserved.
 *
 * Rust conversion of blackmagic/src/target/riscv_debug.c, made for the Swindle
 * firmware. This file is a translation of that C source into Rust and stays
 * under the same license as the original Black Magic code reproduced below.
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

//! RISC-V Debug Module (DM) 0.13 controller.
//!
//! Handles DM activation, hart selection, abstract command execution with
//! timeout protection (patch 94), error clearing, and autoexec configuration.

use super::dmi::{DmiError, DmiTransport};

// --- DM Register Addresses ---
pub const RV_DM_DATA0: u8 = 0x04;
pub const RV_DM_DATA1: u8 = 0x05;
pub const RV_DM_CONTROL: u8 = 0x10;
pub const RV_DM_STATUS: u8 = 0x11;
pub const RV_DM_HART_INFO: u8 = 0x12;
pub const RV_DM_ABSTRACTCS: u8 = 0x16;
pub const RV_DM_COMMAND: u8 = 0x17;
pub const RV_DM_ABSTRACTAUTO: u8 = 0x18;
pub const RV_DM_PROGBUF0: u8 = 0x20;
pub const RV_DM_SBCS: u8 = 0x38;
pub const RV_DM_SBADDRESS0: u8 = 0x39;
pub const RV_DM_SBDATA0: u8 = 0x3c;

// --- DM Control Bits ---
pub const RV_DM_CTRL_ACTIVE: u32 = 1 << 0;
pub const RV_DM_CTRL_NDM_RESET: u32 = 1 << 1;
pub const RV_DM_CTRL_CLR_RESET_HALT_REQ: u32 = 1 << 2;
pub const RV_DM_CTRL_SET_RESET_HALT_REQ: u32 = 1 << 3;
pub const RV_DM_CTRL_HART_SEL_LO_SHIFT: u32 = 16;
pub const RV_DM_CTRL_HART_SEL_LO_MASK: u32 = 0x3ff << 16;
pub const RV_DM_CTRL_HART_SEL_HI_SHIFT: u32 = 6;
pub const RV_DM_CTRL_HART_SEL_HI_MASK: u32 = 0x3ff << 6;
pub const RV_DM_CTRL_HART_SEL_MASK: u32 = RV_DM_CTRL_HART_SEL_LO_MASK | RV_DM_CTRL_HART_SEL_HI_MASK;
pub const RV_DM_CTRL_HART_ACK_RESET: u32 = 1 << 28;
pub const RV_DM_CTRL_HART_RESUME_REQ: u32 = 1 << 30;
pub const RV_DM_CTRL_HALT_REQ: u32 = 1 << 31;

// --- DM Status Bits ---
pub const RV_DM_STAT_VERSION_MASK: u32 = 0x0f;
pub const RV_DM_STAT_VERSION_0_13: u32 = 2;
pub const RV_DM_STAT_VERSION_1_0: u32 = 3;
pub const RV_DM_STAT_AUTHENTICATED: u32 = 1 << 7;
pub const RV_DM_STAT_ANY_HALTED: u32 = 1 << 8;
pub const RV_DM_STAT_ALL_HALTED: u32 = 1 << 9;
pub const RV_DM_STAT_ANY_RUNNING: u32 = 1 << 10;
pub const RV_DM_STAT_ALL_RUNNING: u32 = 1 << 11;
pub const RV_DM_STAT_ANY_UNAVAIL: u32 = 1 << 12;
pub const RV_DM_STAT_ALL_UNAVAIL: u32 = 1 << 13;
pub const RV_DM_STAT_ANY_NONEXIST: u32 = 1 << 14;
pub const RV_DM_STAT_ALL_NONEXIST: u32 = 1 << 15;
pub const RV_DM_STAT_ANY_RESUMEACK: u32 = 1 << 16;
pub const RV_DM_STAT_ALL_RESUMEACK: u32 = 1 << 17;
pub const RV_DM_STAT_ANY_HAVERESET: u32 = 1 << 18;
pub const RV_DM_STAT_ALL_HAVERESET: u32 = 1 << 19;

// --- Abstract CS Bits ---
pub const RV_DM_ABSTCS_DATACOUNT_MASK: u32 = 0x0f;
pub const RV_DM_ABSTCS_CMDERR_SHIFT: u32 = 8;
pub const RV_DM_ABSTCS_CMDERR_MASK: u32 = 0x07 << 8;
pub const RV_DM_ABSTCS_BUSY: u32 = 1 << 12;
pub const RV_DM_ABSTCS_PROGBUF_SIZE_SHIFT: u32 = 24;
pub const RV_DM_ABSTCS_PROGBUF_SIZE_MASK: u32 = 0x1f << 24;

// Abstract Command Error codes
pub const RV_ABST_CMDERR_NONE: u32 = 0;
pub const RV_ABST_CMDERR_BUSY: u32 = 1;
pub const RV_ABST_CMDERR_NOT_SUPP: u32 = 2;
pub const RV_ABST_CMDERR_EXCEPTION: u32 = 3;
pub const RV_ABST_CMDERR_WRONG_STATE: u32 = 4;
pub const RV_ABST_CMDERR_BUS_ERROR: u32 = 5;
pub const RV_ABST_CMDERR_OTHER: u32 = 7;

// --- Abstract Command Construction ---
pub const RV_CMD_TYPE_ACCESS_REG: u32 = 0 << 24;
pub const RV_CMD_AARSIZE_32: u32 = 2 << 20;
pub const RV_CMD_POSTEXEC: u32 = 1 << 18;
pub const RV_CMD_TRANSFER: u32 = 1 << 17;
pub const RV_CMD_WRITE: u32 = 1 << 16;

/// Helper: encode `hartsel` into `dmcontrol` bits.
pub fn hartsel_mask(hart_idx: u32) -> u32 {
    let lo = (hart_idx & 0x3ff) << RV_DM_CTRL_HART_SEL_LO_SHIFT;
    let hi = ((hart_idx >> 10) & 0x3ff) << RV_DM_CTRL_HART_SEL_HI_SHIFT;
    lo | hi
}

/// Abstract command builder for 32-bit register access.
pub fn make_reg_access_cmd(regno: u16, write: bool, transfer: bool, postexec: bool) -> u32 {
    let mut cmd = RV_CMD_TYPE_ACCESS_REG | RV_CMD_AARSIZE_32 | (regno as u32);
    if write {
        cmd |= RV_CMD_WRITE;
    }
    if transfer {
        cmd |= RV_CMD_TRANSFER;
    }
    if postexec {
        cmd |= RV_CMD_POSTEXEC;
    }
    cmd
}

/// Poll abstract command completion with bounded iterations (patch 94 timeout protection).
pub fn wait_command_complete(dmi: &mut dyn DmiTransport) -> Result<(), DmiError> {
    // 2,000 iterations @ ~10-50us = 20-100ms max timeout
    for _ in 0..2000 {
        let cs = dmi.read(RV_DM_ABSTRACTCS)?;
        if (cs & RV_DM_ABSTCS_BUSY) == 0 {
            let cmderr = (cs & RV_DM_ABSTCS_CMDERR_MASK) >> RV_DM_ABSTCS_CMDERR_SHIFT;
            if cmderr == RV_ABST_CMDERR_NONE {
                return Ok(());
            } else {
                // Clear the error by writing 1s to the cmderr field
                let _ = dmi.write(RV_DM_ABSTRACTCS, cs);
                return Err(DmiError::Fault);
            }
        }
    }
    // Timeout: try to clear busy/cmderr
    let cs = dmi.read(RV_DM_ABSTRACTCS).unwrap_or(0);
    let _ = dmi.write(RV_DM_ABSTRACTCS, cs);
    Err(DmiError::Timeout)
}

/// Execute an abstract command and wait for completion.
pub fn exec_command(dmi: &mut dyn DmiTransport, cmd: u32) -> Result<(), DmiError> {
    // Clear any previous error before launching command
    let cs = dmi.read(RV_DM_ABSTRACTCS)?;
    if (cs & RV_DM_ABSTCS_CMDERR_MASK) != 0 {
        dmi.write(RV_DM_ABSTRACTCS, cs)?;
    }
    dmi.write(RV_DM_COMMAND, cmd)?;
    wait_command_complete(dmi)
}
