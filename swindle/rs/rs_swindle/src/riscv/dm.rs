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

use super::dm_regs::*;
use super::dmi::{DmiError, DmiTransport};

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
