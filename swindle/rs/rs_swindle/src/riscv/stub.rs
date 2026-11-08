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
use crate::stub_exit::STUB_EXIT_OK;

use crate::target::driver::HaltReason;

/// Per-session counters, printed by `mon stub_stats` (and once by the flash engine's
/// FLASHSTAT line, when that debug line is enabled).
pub static STUB_STAT_RUNS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
pub static STUB_STAT_HALTED: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
pub static STUB_STAT_WAIT_MS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
pub static STUB_STAT_HOLD_MS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
pub static STUB_STAT_PC: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
pub static STUB_STAT_SP: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
pub static STUB_STAT_CODE0: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
pub static STUB_STAT_CODE1: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
/// Blocks a driver had to program through its register path because the stub's work did not
/// verify (`write_stub()` -> erase + `write_direct()`): the fallback that makes a load look
/// successful while the fast path was broken.
pub static STUB_STAT_REPAIRS: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

/// One block went through the driver's register-repair path (`mon stub_stats` repairs).
pub fn stub_stat_repair() {
    stub_stat_bump(&STUB_STAT_REPAIRS, 1);
}

/// The counters in a stable order, for `mon stub_stats`:
/// `[runs, halted, repairs, wait_ms, hold_ms, pc, sp, code0, code1]`.
pub fn stub_stats() -> [u32; 9] {
    [
        STUB_STAT_RUNS.load(Ordering::Relaxed),
        STUB_STAT_HALTED.load(Ordering::Relaxed),
        STUB_STAT_REPAIRS.load(Ordering::Relaxed),
        STUB_STAT_WAIT_MS.load(Ordering::Relaxed),
        STUB_STAT_HOLD_MS.load(Ordering::Relaxed),
        STUB_STAT_PC.load(Ordering::Relaxed),
        STUB_STAT_SP.load(Ordering::Relaxed),
        STUB_STAT_CODE0.load(Ordering::Relaxed),
        STUB_STAT_CODE1.load(Ordering::Relaxed),
    ]
}

/// Zero the counters (`mon stub_stats reset`), so a test can read one session's worth.
pub fn stub_stats_reset() {
    for counter in [
        &STUB_STAT_RUNS,
        &STUB_STAT_HALTED,
        &STUB_STAT_WAIT_MS,
        &STUB_STAT_HOLD_MS,
        &STUB_STAT_PC,
        &STUB_STAT_SP,
        &STUB_STAT_CODE0,
        &STUB_STAT_CODE1,
        &STUB_STAT_REPAIRS,
    ] {
        counter.store(0, Ordering::Relaxed);
    }
}

/// How long between two `dmstatus` samples while a stub runs, in milliseconds.
const STUB_POLL_INTERVAL_MS: u32 = 1;

/// Per-page stub hold override, in ms, for `mon stub_hold` sweeps (`u32::MAX` = unset).
///
/// A flash driver computes its hold itself, from the part family (see
/// `stub_hold_per_page()` in `rs_ch32v00x`). This lets an experiment replace that
/// per-page number at runtime instead of rebuilding the probe once per value, which is
/// what a sweep of the hold over a part's silicon needs: the number is a property of the
/// part's flash programming time, and it is only visible as "does the page come out
/// right", so a sweep has to program real pages at each value (see doc/ch32v00x.md).
pub static STUB_HOLD_OVERRIDE_MS: AtomicU32 = AtomicU32::new(u32::MAX);

/// Use `ms` per hardware page instead of the driver's own hold; `None` goes back to it.
pub fn set_stub_hold_override_ms(ms: Option<u32>) {
    STUB_HOLD_OVERRIDE_MS.store(ms.unwrap_or(u32::MAX), Ordering::Relaxed);
}

/// The per-page hold override a sweep set, if any.
pub fn stub_hold_override_ms() -> Option<u32> {
    match STUB_HOLD_OVERRIDE_MS.load(Ordering::Relaxed) {
        u32::MAX => None,
        ms => Some(ms),
    }
}

/// What `timeout: None` means: one second of polling.
///
/// Every stub in this tree finishes in milliseconds, so a second is generous; a caller
/// that knows its stub is slower passes `Some(ms)`.
const STUB_TIMEOUT_DEFAULT_MS: u32 = 1000;

#[cfg(not(feature = "hosted"))]
use rust_esprit::delay_ms;
#[cfg(feature = "hosted")]
fn delay_ms(_a: u32) {
    std::thread::sleep(core::time::Duration::from_millis(_a as u64));
}

use core::sync::atomic::{AtomicU32, Ordering};


fn restore_state(
    hart: &mut RiscvHart,
    saved_mie: u32,
    saved_pc: u32,
    saved_sp: Option<u32>,
) {
    let _ = hart.csr_write(RV_CSR_MIE, saved_mie);
    let _ = hart.pc_write(saved_pc);
    if let Some(sp) = saved_sp {
        let _ = hart.gpr_write(RV_REG_SP, sp);
    }
}

/// Internal flash stub execution engine.
///
/// If `sp` is `Some(val)` and non-zero:
/// - Current SP is saved
/// - SP is set to `val`
/// - At exit, SP is restored to its original saved value.
/// If `sp` is `None`: SP is untouched.
/// Best-effort counter bump.
///
/// `fetch_add` needs an atomic read-modify-write, which Cortex-M0+ (the RP2040 build)
/// does not have, so the statistics are done as load+store: a flash session touches
/// them from one context only, and these counters only ever feed a log line.
fn stub_stat_bump(counter: &AtomicU32, by: u32) {
    let value = counter.load(Ordering::Relaxed).wrapping_add(by);
    counter.store(value, Ordering::Relaxed);
}

fn run_stub_internal(
    hart: &mut RiscvHart,
    loadaddr: u32,
    sp: Option<u32>,
    param1: u32,
    param2: u32,
    param3: u32,
    param4: u32,
    timeout: Option<u32>,
    hold_ms: u32,
) -> bool {
    // The hart has to be halted before its state can be saved or steered.
    let halt_res = hart.halt_request();
    if halt_res.is_err() {
        return false;
    }

    // 1. Save PC, MIE, and optionally SP
    let saved_pc = hart.pc_read().unwrap_or(0);
    let saved_mie = hart.csr_read(RV_CSR_MIE).unwrap_or(0);
    let saved_sp = sp.map(|_| hart.gpr_read(RV_REG_SP).unwrap_or(0));

    // 2. Disable interrupts during stub execution
    let mut ok = hart.csr_write(RV_CSR_MIE, 0).is_ok();

    // 3. Set SP if specified and non-zero
    if let Some(sp_val) = sp {
        if sp_val != 0 {
            ok = ok && hart.gpr_write(RV_REG_SP, sp_val).is_ok();
        }
    }

    // 4. Load parameters into a0..a3
    ok = ok
        && hart.gpr_write(RV_REG_A0, param1).is_ok()
        && hart.gpr_write(RV_REG_A1, param2).is_ok()
        && hart.gpr_write(RV_REG_A2, param3).is_ok()
        && hart.gpr_write(RV_REG_A3, param4).is_ok();

    // 5. Set PC to stub entry point and resume execution (halt_resume ensures ebreakm)
    if !ok || hart.pc_write(loadaddr).is_err() || hart.halt_resume(false).is_err() {
        restore_state(hart, saved_mie, saved_pc, saved_sp);
        return false;
    }

    // 6. Let the stub work, then watch for the breakpoint.
    //
    // `hold_ms` is a period of *silence* after the resume: a CH32V00x corrupts its own
    // flash programming if its debug module is accessed while its stub is running, so a
    // platform whose stub needs this passes a hold long enough for the work to finish.
    // The hold is outside the timeout - the timeout is what the polling may take.
    // After it, `dmstatus` is sampled every `STUB_POLL_INTERVAL_MS` until the stub
    // reports its breakpoint or the budget runs out.
    stub_stat_bump(&STUB_STAT_RUNS, 1);
    stub_stat_bump(&STUB_STAT_HOLD_MS, hold_ms);
    if hold_ms != 0u32 {
        delay_ms(hold_ms);
    }

    let budget_ms = timeout.unwrap_or(STUB_TIMEOUT_DEFAULT_MS);
    let mut success = false;
    let mut waited_ms = 0u32;
    let mut fast_polls = 0u32;
    loop {
        let (reason, _) = hart.halt_poll();
        match reason {
            HaltReason::Breakpoint => {
                success = true;
                break;
            }
            HaltReason::Running => {}
            _ => break,
        }
        if fast_polls < 16 {
            fast_polls += 1;
            continue;
        }
        if waited_ms >= budget_ms {
            break;
        }
        delay_ms(STUB_POLL_INTERVAL_MS);
        waited_ms += STUB_POLL_INTERVAL_MS;
    }

    stub_stat_bump(&STUB_STAT_WAIT_MS, waited_ms);
    if success {
        stub_stat_bump(&STUB_STAT_HALTED, 1);
    }

    if !success {
        // Stop the core if it did not report its breakpoint, then dump diagnostic state
        let halt_res = hart.halt_request();
        let (after_halt, _) = hart.halt_poll();
        let pc = hart.pc_read().unwrap_or(u32::MAX);
        let sp = hart.gpr_read(RV_REG_SP).unwrap_or(u32::MAX);
        let a0 = hart.gpr_read(RV_REG_A0).unwrap_or(u32::MAX);
        let dcsr = hart.csr_read(RV_CSR_DCSR).unwrap_or(u32::MAX);
        let mepc = hart.csr_read(RV_CSR_MEPC).unwrap_or(u32::MAX);
        let mcause = hart.csr_read(RV_CSR_MCAUSE).unwrap_or(u32::MAX);
        let mtval = hart.csr_read(RV_CSR_MTVAL).unwrap_or(u32::MAX);
        let mut code = [0u8; 8];
        let _ = hart.mem_read(pc & !1u32, &mut code);
        let code0 = u32::from_le_bytes([code[0], code[1], code[2], code[3]]);
        let code1 = u32::from_le_bytes([code[4], code[5], code[6], code[7]]);
        STUB_STAT_PC.store(pc, Ordering::Relaxed);
        STUB_STAT_SP.store(sp, Ordering::Relaxed);
        STUB_STAT_CODE0.store(code0, Ordering::Relaxed);
        STUB_STAT_CODE1.store(code1, Ordering::Relaxed);
        crate::bmpwarning!(
            "STUBNOSTOP halt_ok={} after_halt={} pc={} sp={} a0={} dcsr={} mepc={} mcause={} mtval={} code={},{}\n",
            u32::from(halt_res.is_ok()),
            after_halt as u32,
            pc,
            sp,
            a0,
            dcsr,
            mepc,
            mcause,
            mtval,
            code0,
            code1
        );
    }

    // 7. Verify the exit code in a0 (STUB_EXIT_OK = the stub did its work)
    if success {
        let a0 = hart.gpr_read(RV_REG_A0).unwrap_or(0xffff_ffff);
        success = a0 == u32::from(STUB_EXIT_OK);
    }

    // 8. Restore MIE, PC, and SP
    restore_state(hart, saved_mie, saved_pc, saved_sp);

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
    run_stub_internal(hart, loadaddr, None, param1, param2, param3, param4, None, 0)
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
    run_stub_internal(
        hart,
        loadaddr,
        Some(stack),
        param1,
        param2,
        param3,
        param4,
        None,
        0,
    )
}

/// [`run_stub_with_stack`] with an explicit bound on how long the polling may take, in
/// milliseconds; `None` means [`STUB_TIMEOUT_DEFAULT_MS`].
pub fn run_stub_with_stack_timeout(
    hart: &mut RiscvHart,
    loadaddr: u32,
    stack: u32,
    param1: u32,
    param2: u32,
    param3: u32,
    param4: u32,
    timeout: Option<u32>,
) -> bool {
    run_stub_internal(
        hart,
        loadaddr,
        Some(stack),
        param1,
        param2,
        param3,
        param4,
        timeout,
        0,
    )
}

/// [`run_stub_with_stack_timeout`] plus a `hold`, in milliseconds.
///
/// The target is left completely alone for `hold_ms` after the resume, and only then are
/// `dmstatus` reads started. A CH32V00x corrupts its own flash programming when its debug
/// module is touched while its stub runs (measured on a CH32V006: `dmstatus` polled every
/// 2 ms -> 5/5 corrupt pages, the same accesses to `abstractcs` -> 0/5, `dmstatus` after
/// the stub had finished -> 0/5), so its page-write stub is started with a few
/// milliseconds of silence; by then the work (~1 ms) is done and the first poll sees the
/// breakpoint.
pub fn run_stub_with_stack_hold(
    hart: &mut RiscvHart,
    loadaddr: u32,
    stack: u32,
    param1: u32,
    param2: u32,
    param3: u32,
    param4: u32,
    timeout: Option<u32>,
    hold_ms: u32,
) -> bool {
    run_stub_internal(
        hart,
        loadaddr,
        Some(stack),
        param1,
        param2,
        param3,
        param4,
        timeout,
        hold_ms,
    )
}

/// Run a stub with an explicit stack pointer and return the value it left in `a2`.
///
/// Stubs that compute a result publish it in `a2` by convention (see
/// `ch32v3x_crc32.c`); the runner only restores MIE/PC/SP, so `a2` survives the call.
/// `None` means the stub did not report [`STUB_EXIT_OK`] in `a0`, so no result is ever
/// taken from a failed run.
pub fn run_stub_with_stack_ret(
    hart: &mut RiscvHart,
    loadaddr: u32,
    stack: u32,
    param1: u32,
    param2: u32,
    param3: u32,
    param4: u32,
) -> Option<u32> {
    run_stub_with_stack_result(hart, loadaddr, stack, param1, param2, param3, param4)
}

/// Run a stub with an explicit stack pointer and return the value it left in `a2`.
///
/// Alias of [`run_stub_with_stack_ret`], consistent with Cortex-M's `run_stub_with_stack_result`.
pub fn run_stub_with_stack_result(
    hart: &mut RiscvHart,
    loadaddr: u32,
    stack: u32,
    param1: u32,
    param2: u32,
    param3: u32,
    param4: u32,
) -> Option<u32> {
    if !run_stub_internal(
        hart,
        loadaddr,
        Some(stack),
        param1,
        param2,
        param3,
        param4,
        None,
        0,
    ) {
        return None;
    }
    hart.gpr_read(RV_REG_A2).ok()
}
