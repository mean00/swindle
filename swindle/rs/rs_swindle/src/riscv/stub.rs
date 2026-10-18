//! RISC-V flash stub runner (RV32E / RV32I).
//!
//! Executes on-target machine code routines (flash erase/write/CRC) and
//! monitors completion via breakpoint halt.

use super::dmi::DmiTransport;
use super::hart::RiscvHart;
use crate::target::driver::HaltReason;

pub const RV_REG_SP: u8 = 2;
pub const RV_REG_A0: u8 = 10;
pub const RV_REG_A1: u8 = 11;
pub const RV_REG_A2: u8 = 12;
pub const RV_REG_A3: u8 = 13;
pub const RV_CSR_MIE: u16 = 0x304;

#[cfg(not(feature = "hosted"))]
use rust_esprit::delay_ms;
#[cfg(feature = "hosted")]
fn delay_ms(_a: u32) {
    std::thread::sleep(core::time::Duration::from_millis(_a as u64));
}

/// Run a flash/utility stub on target.
///
/// Parameters:
/// - `loadaddr`: Entry point address of stub in target SRAM.
/// - `param1`..`param4`: Parameters loaded into `a0`, `a1`, `a2`, `a3`.
///   `param4` is also used as stack pointer `sp` if non-zero.
///
/// Returns `true` if the stub ran to completion (halted on ebreak) and returned `a0 == 0`.
pub fn run_stub(
    hart: &RiscvHart,
    dmi: &mut dyn DmiTransport,
    loadaddr: u32,
    param1: u32,
    param2: u32,
    param3: u32,
    param4: u32,
) -> bool {
    // 1. Save PC, SP and MIE
    let saved_pc = hart.pc_read(dmi).unwrap_or(0);
    let saved_sp = hart.gpr_read(dmi, RV_REG_SP).unwrap_or(0);
    let saved_mie = hart.csr_read(dmi, RV_CSR_MIE).unwrap_or(0);

    // 2. Disable interrupts during stub execution
    let _ = hart.csr_write(dmi, RV_CSR_MIE, 0);

    // 3. Load parameters into a0..a3, and set SP if param4 provided
    let _ = hart.gpr_write(dmi, RV_REG_A0, param1);
    let _ = hart.gpr_write(dmi, RV_REG_A1, param2);
    let _ = hart.gpr_write(dmi, RV_REG_A2, param3);
    let _ = hart.gpr_write(dmi, RV_REG_A3, param4);
    if param4 != 0 {
        let _ = hart.gpr_write(dmi, RV_REG_SP, param4);
    }

    // 4. Set PC to stub entry point
    if hart.pc_write(dmi, loadaddr).is_err() {
        let _ = hart.csr_write(dmi, RV_CSR_MIE, saved_mie);
        let _ = hart.pc_write(dmi, saved_pc);
        if param4 != 0 && saved_sp != 0 {
            let _ = hart.gpr_write(dmi, RV_REG_SP, saved_sp);
        }
        return false;
    }

    // 5. Resume execution
    if hart.halt_resume(dmi, false).is_err() {
        let _ = hart.csr_write(dmi, RV_CSR_MIE, saved_mie);
        let _ = hart.pc_write(dmi, saved_pc);
        if param4 != 0 && saved_sp != 0 {
            let _ = hart.gpr_write(dmi, RV_REG_SP, saved_sp);
        }
        return false;
    }

    // 6. Wait for breakpoint or request halt (timeout ~10s)
    let mut success = false;
    for _ in 0..10_000 {
        let (reason, _) = hart.halt_poll(dmi);
        match reason {
            HaltReason::Breakpoint | HaltReason::Request => {
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
    let _ = hart.halt_request(dmi);

    // 7. Verify return code in a0 (0 = success)
    if success {
        let a0 = hart.gpr_read(dmi, RV_REG_A0).unwrap_or(0xffff_ffff);
        success = a0 == 0;
    }

    // 8. Restore MIE, PC, and SP
    let _ = hart.csr_write(dmi, RV_CSR_MIE, saved_mie);
    let _ = hart.pc_write(dmi, saved_pc);
    if param4 != 0 && saved_sp != 0 {
        let _ = hart.gpr_write(dmi, RV_REG_SP, saved_sp);
    }

    success
}
