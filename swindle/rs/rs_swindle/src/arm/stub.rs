/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2012-2020  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>,
 * Copyright (C) 2022-2023 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 * Koen De Vleeschauwer and Uwe Bonnes
 *
 * Rust conversion of blackmagic/src/target/cortexm.c (cortexm_run_stub), made
 * for the Swindle firmware. This file is a translation of that C source into
 * Rust and stays under the same license as the original Black Magic code
 * reproduced below.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

//! Cortex-M flash stub runner.
//!
//! Executes on-target machine code routines (flash erase/write/CRC) and monitors
//! completion through the `bkpt` instruction the stub traps on.

use crate::stub_exit::{STUB_EXIT_OK, STUB_TIMEOUT_MS};
use crate::target::{CortexMAccess, HaltReason};

#[cfg(not(feature = "hosted"))]
use rust_esprit::delay_ms;
#[cfg(feature = "hosted")]
fn delay_ms(_a: u32) {
    std::thread::sleep(core::time::Duration::from_millis(_a as u64));
}

/// Index of `r0` in the Cortex-M register file (C `CORTEX_REG_R0`); the stub arguments
/// follow in `r1..r3`.
const CORTEX_REG_R0: usize = 0;
/// Index of `sp` in the register file (C `CORTEX_REG_SP`).
const CORTEX_REG_SP: usize = 13;
/// Index of `pc` in the register file (C `CORTEX_REG_PC`).
const CORTEX_REG_PC: usize = 15;
/// Index of `xpsr` in the register file (C `CORTEX_REG_XPSR`).
const CORTEX_REG_XPSR: usize = 16;
/// Index of `msp` in the register file (C `CORTEX_REG_MSP`).
const CORTEX_REG_MSP: usize = 17;
/// Index of the packed `control`/`faultmask`/`basepri`/`primask` register (C `CORTEX_REG_SPECIAL`).
const CORTEX_REG_SPECIAL: usize = 19;
/// Indices for TrustZone banked stack and stack-limit registers in `REGNUM_CORTEX_M_TRUSTZONE`.
/// In Cortex-M33 / ARMv8-M, hardware stack limits (MSPLIM/PSPLIM) must be cleared to 0
/// when running stubs in scratch SRAM to prevent stack limit UsageFaults.
const CORTEX_REG_MSP_NS: usize = 20;
#[allow(dead_code)]
const CORTEX_REG_PSP_NS: usize = 21;
const CORTEX_REG_MSPLIM_NS: usize = 22;
const CORTEX_REG_PSPLIM_NS: usize = 23;
const CORTEX_REG_MSP_S: usize = 25;
#[allow(dead_code)]
const CORTEX_REG_PSP_S: usize = 26;
const CORTEX_REG_MSPLIM_S: usize = 27;
const CORTEX_REG_PSPLIM_S: usize = 28;

/// `xPSR.T`: the stub is Thumb code, so the core must resume in Thumb state.
const CORTEXM_XPSR_THUMB: u32 = 1 << 24;
/// Value written to the special register before a stub runs. Kept verbatim from the C runner
/// (`regs[19] = 1`, commented there as masking all interrupts) so the Rust path has the same
/// effect on the core.
const CORTEXM_STUB_SPECIAL: u32 = 1;
/// Encoded `bkpt` with immediate 0 (C `CORTEX_THUMB_BKPT`), the opcode the stubs trap with; the
/// immediate carries the stub's result, see [`run_stub`] / [`run_stub_ret`].
const CORTEX_THUMB_BKPT: u16 = 0xbe00;

/// Read the program counter (C `cortexm_pc_read()`): `DCRSR` selector `0x0f` through `DCRDR`,
/// which is what a core register read of register 15 does.
fn pc_read(cm: &CortexMAccess) -> Option<u32> {
    let mut buf = [0u8; 4];
    if !cm.reg_read(CORTEX_REG_PC as u32, &mut buf) {
        return None;
    }
    Some(u32::from_le_bytes(buf))
}

/// Run the stub at `loadaddr` with `r0..r3` and report whether it succeeded.
///
/// `true` means the stub ran to completion and published [`STUB_EXIT_OK`] (`bkpt #1`) at
/// its trap instruction; anything else -- a stub that reported failure
/// ([`crate::stub_exit::STUB_EXIT_FAIL`]), a failed run, a timeout -- is `false`. Stubs that
/// also return a value use [`run_stub_result`] so the value is only read after a successful exit.
pub fn run_stub(cm: &CortexMAccess, loadaddr: u32, r0: u32, r1: u32, r2: u32, r3: u32) -> bool {
    run_stub_ret(cm, loadaddr, r0, r1, r2, r3) == Some(STUB_EXIT_OK)
}

/// Run the stub at `loadaddr` with an explicit stack pointer and `r0..r3`, reporting whether it succeeded.
pub fn run_stub_with_stack(
    cm: &CortexMAccess,
    loadaddr: u32,
    stack: u32,
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
) -> bool {
    run_stub_ret_with_stack(cm, loadaddr, stack, r0, r1, r2, r3) == Some(STUB_EXIT_OK)
}

/// Register a stub publishes a computed result in, unless the stub documents another one
/// (`r1` in every stub Swindle ships, matching the C drivers' `target_reg_read(target, 1U, ...)`).
pub const STUB_RESULT_REG: u32 = 1;

/// Run the stub at `loadaddr` with `r0..r3` and return the value it left in `result_reg`.
///
/// `None` means the stub did not report [`STUB_EXIT_OK`] (or the register could not be read),
/// so a result is never taken from a failed run.
pub fn run_stub_result(
    cm: &CortexMAccess,
    loadaddr: u32,
    result_reg: u32,
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
) -> Option<u32> {
    if !run_stub(cm, loadaddr, r0, r1, r2, r3) {
        return None;
    }
    reg_read_u32(cm, result_reg)
}

/// Run the stub at `loadaddr` with an explicit stack pointer and `r0..r3`, returning the
/// value it left in `result_reg`; `None` means the run failed (see [`run_stub_result`]).
pub fn run_stub_with_stack_result(
    cm: &CortexMAccess,
    loadaddr: u32,
    stack: u32,
    result_reg: u32,
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
) -> Option<u32> {
    if !run_stub_with_stack(cm, loadaddr, stack, r0, r1, r2, r3) {
        return None;
    }
    reg_read_u32(cm, result_reg)
}

/// Read one register as a `u32` (`target_reg_read()`), `None` on a read error.
fn reg_read_u32(cm: &CortexMAccess, reg: u32) -> Option<u32> {
    let mut buf = [0u8; 4];
    if !cm.reg_read(reg, &mut buf) {
        return None;
    }
    Some(u32::from_le_bytes(buf))
}


/// Run the stub at `loadaddr` with `r0..r3` and return the `bkpt` immediate it trapped with.
fn run_stub_ret(
    cm: &CortexMAccess,
    loadaddr: u32,
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
) -> Option<u8> {
    run_stub_ret_internal(cm, loadaddr, None, r0, r1, r2, r3)
}

/// Run the stub at `loadaddr` with an explicit stack pointer and `r0..r3`, returning the `bkpt` immediate.
fn run_stub_ret_with_stack(
    cm: &CortexMAccess,
    loadaddr: u32,
    stack: u32,
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
) -> Option<u8> {
    run_stub_ret_internal(cm, loadaddr, Some(stack), r0, r1, r2, r3)
}

fn run_stub_ret_internal(
    cm: &CortexMAccess,
    loadaddr: u32,
    stack: Option<u32>,
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
) -> Option<u8> {
    if cm.target().is_null() {
        return None;
    }

    // Set argument registers r0..r3 directly
    cm.reg_write(CORTEX_REG_R0 as u32, &r0.to_le_bytes());
    cm.reg_write((CORTEX_REG_R0 + 1) as u32, &r1.to_le_bytes());
    cm.reg_write((CORTEX_REG_R0 + 2) as u32, &r2.to_le_bytes());
    cm.reg_write((CORTEX_REG_R0 + 3) as u32, &r3.to_le_bytes());

    if let Some(sp_val) = stack {
        if sp_val != 0 {
            cm.reg_write(CORTEX_REG_SP as u32, &sp_val.to_le_bytes());
            cm.reg_write(CORTEX_REG_MSP as u32, &sp_val.to_le_bytes());
            cm.reg_write(CORTEX_REG_MSP_NS as u32, &sp_val.to_le_bytes());
            cm.reg_write(CORTEX_REG_MSP_S as u32, &sp_val.to_le_bytes());
            cm.reg_write(CORTEX_REG_MSPLIM_NS as u32, &0u32.to_le_bytes());
            cm.reg_write(CORTEX_REG_MSPLIM_S as u32, &0u32.to_le_bytes());
            cm.reg_write(CORTEX_REG_PSPLIM_NS as u32, &0u32.to_le_bytes());
            cm.reg_write(CORTEX_REG_PSPLIM_S as u32, &0u32.to_le_bytes());
        }
    }
    cm.reg_write(CORTEX_REG_PC as u32, &loadaddr.to_le_bytes());
    cm.reg_write(CORTEX_REG_XPSR as u32, &CORTEXM_XPSR_THUMB.to_le_bytes());
    cm.reg_write(CORTEX_REG_SPECIAL as u32, &CORTEXM_STUB_SPECIAL.to_le_bytes());

    if cm.check_error() {
        return None;
    }

    // Execute the stub.
    cm.halt_resume(false);
    if cm.check_error() {
        return None;
    }

    let mut reason = HaltReason::Running;
    let mut timed_out = true;
    for i in 0..(STUB_TIMEOUT_MS + 16) {
        reason = cm.halt_poll().0;
        if reason != HaltReason::Running {
            timed_out = false;
            break;
        }
        if i >= 16 {
            delay_ms(1);
        }
    }

    if timed_out {
        // Yank a hung stub back so the core is left halted rather than spinning.
        cm.halt_request();
        let pc = pc_read(cm).unwrap_or(0);
        let cfsr = cm.read32(crate::arm::cortexm_registers::CORTEXM_CFSR);
        let hfsr = cm.read32(crate::arm::cortexm_registers::CORTEXM_HFSR);
        crate::bmpwarning!("stub timed out! pc=0x{:x} cfsr=0x{:x} hfsr=0x{:x}\n", pc, cfsr, hfsr);
        return None;
    }
    if reason != HaltReason::Breakpoint {
        let pc = pc_read(cm).unwrap_or(0);
        let cfsr = cm.read32(crate::arm::cortexm_registers::CORTEXM_CFSR);
        let hfsr = cm.read32(crate::arm::cortexm_registers::CORTEXM_HFSR);
        crate::bmpwarning!("stub not breakpoint! reason={:?}, pc=0x{:x} cfsr=0x{:x} hfsr=0x{:x}\n", reason as u32, pc, cfsr, hfsr);
        return None;
    }

    let Some(pc) = pc_read(cm) else {
        crate::bmpwarning!("stub failed to read pc!\n");
        return None;
    };
    let instruction = cm.read16(pc);
    if instruction & 0xff00 != CORTEX_THUMB_BKPT {
        crate::bmpwarning!("stub not bkpt insn! pc=0x{:x}, insn=0x{:x}\n", pc, instruction);
        return None;
    }

    let imm = (instruction & 0xff) as u8;
    if imm != STUB_EXIT_OK {
        crate::bmpwarning!("stub bkpt imm != STUB_EXIT_OK! imm={}\n", imm as u32);
    }
    Some(imm)
}
