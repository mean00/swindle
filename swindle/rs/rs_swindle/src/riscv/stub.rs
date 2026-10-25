/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2023 1BitSquared <info@1bitsquared.com>
 * Written by Rachel Mant <git@dragonmux.network>
 * All rights reserved.
 *
 * Rust conversion of blackmagic/src/target/riscv32.c (riscv32_run_stub), made
 * for the Swindle firmware. This file is a translation of that C source into
 * Rust and stays under the same license as the original Black Magic code
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

//! RISC-V flash stub runner (RV32E / RV32I).
//!
//! Executes on-target machine code routines (flash erase/write/CRC) and
//! monitors completion via breakpoint halt.

// use super::dmi::DmiTransport;
use super::hart::RiscvHart;
use super::rv_csr::*;
use crate::target::driver::HaltReason;

#[cfg(not(feature = "hosted"))]
use rust_esprit::delay_ms;
#[cfg(feature = "hosted")]
fn delay_ms(_a: u32) {
    std::thread::sleep(core::time::Duration::from_millis(_a as u64));
}


/// Internal flash stub execution engine.
///
/// If `sp` is `Some(val)` and non-zero:
/// - Current SP is saved
/// - SP is set to `val`
/// - At exit, SP is restored to its original saved value.
/// If `sp` is `None`: SP is untouched.
fn run_stub_internal(
    hart: &mut RiscvHart,
    loadaddr: u32,
    sp: Option<u32>,
    param1: u32,
    param2: u32,
    param3: u32,
    param4: u32,
) -> bool {
    // 1. Save PC, MIE, and optionally SP
    let saved_pc = hart.pc_read().unwrap_or(0);
    let saved_mie = hart.csr_read(RV_CSR_MIE).unwrap_or(0);
    let saved_sp = if sp.is_some() {
        hart.gpr_read(RV_REG_SP).unwrap_or(0)
    } else {
        0
    };

    // 2. Disable interrupts during stub execution
    let _ = hart.csr_write(RV_CSR_MIE, 0);

    // 3. Set SP if specified and non-zero
    if let Some(sp_val) = sp {
        if sp_val != 0 {
            let _ = hart.gpr_write(RV_REG_SP, sp_val);
        }
    }

    // 4. Load parameters into a0..a3
    let _ = hart.gpr_write(RV_REG_A0, param1);
    let _ = hart.gpr_write(RV_REG_A1, param2);
    let _ = hart.gpr_write(RV_REG_A2, param3);
    let _ = hart.gpr_write(RV_REG_A3, param4);

    // 5. Set PC to stub entry point
    if hart.pc_write(loadaddr).is_err() {
        let _ = hart.csr_write(RV_CSR_MIE, saved_mie);
        let _ = hart.pc_write(saved_pc);
        if sp.is_some() {
            let _ = hart.gpr_write(RV_REG_SP, saved_sp);
        }
        return false;
    }

    // 6. Resume execution
    if hart.halt_resume(false).is_err() {
        let _ = hart.csr_write(RV_CSR_MIE, saved_mie);
        let _ = hart.pc_write(saved_pc);
        if sp.is_some() {
            let _ = hart.gpr_write(RV_REG_SP, saved_sp);
        }
        return false;
    }

    // 7. Wait for breakpoint (timeout ~10s)
    let mut success = false;
    for _ in 0..10_000 {
        let (reason, _) = hart.halt_poll();
        match reason {
            HaltReason::Breakpoint => {
                success = true;
                break;
            }
            HaltReason::Running => {
                // Short spin delay
                delay_ms(1);
            }
            _ => {
                break;
            }
        }
    }

    // Ensure target is halted
    let _ = hart.halt_request();

    // 8. Verify return code in a0 (0 = success)
    if success {
        let a0 = hart.gpr_read(RV_REG_A0).unwrap_or(0xffff_ffff);
        success = a0 == 0;
    }

    // 9. Restore MIE, PC, and SP
    let _ = hart.csr_write(RV_CSR_MIE, saved_mie);
    let _ = hart.pc_write(saved_pc);
    if sp.is_some() {
        let _ = hart.gpr_write(RV_REG_SP, saved_sp);
    }

    success
}

/// Run a flash/utility stub on target without touching the stack pointer.
pub fn run_stub(
    hart: &mut RiscvHart,
    loadaddr: u32,
    param1: u32,
    param2: u32,
    param3: u32,
    param4: u32,
) -> bool {
    run_stub_internal(hart, loadaddr, None, param1, param2, param3, param4)
}

/// Run a flash/utility stub on target with an explicit stack pointer.
/// If `stack` is non-zero, SP is saved before execution and restored afterwards.
pub fn run_stub_with_stack(
    hart: &mut RiscvHart,
    loadaddr: u32,
    stack: u32,
    param1: u32,
    param2: u32,
    param3: u32,
    param4: u32,
) -> bool {
    run_stub_internal(hart, loadaddr, Some(stack), param1, param2, param3, param4)
}

/// Run a stub with an explicit stack pointer and return the value it left in `a2`.
///
/// Stubs that compute a result publish it in `a2` by convention (see
/// `ch32v3x_crc32.c`); the runner only restores MIE/PC/SP, so `a2` survives the call.
/// `None` means the stub reported failure in `a0`.
pub fn run_stub_with_stack_ret(
    hart: &mut RiscvHart,
    loadaddr: u32,
    stack: u32,
    param1: u32,
    param2: u32,
    param3: u32,
    param4: u32,
) -> Option<u32> {
    if !run_stub_internal(hart, loadaddr, Some(stack), param1, param2, param3, param4) {
        return None;
    }
    hart.gpr_read(RV_REG_A2).ok()
}
