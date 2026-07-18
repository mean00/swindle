/// \brief Standard RISC-V FreeRTOS support
///
/// This module implements the FreeRTOS context switch handling for the standard 
/// RISC-V RV32 layout. 
///
/// Stack layout for standard RV32 (Total = 31 x 32-bit registers):
/// - mepc (offset 0)
/// - ra/x1 (offset 4)
/// - x5..x31 (offsets 8..112)
/// - xCriticalNesting (offset 116)
/// - mstatus (offset 120)

use crate::bmp::{bmp_read_registers, bmp_write_register, bmp_read_register};
use crate::freertos::freertos_trait::freertos_switch_handler;
use super::freertos_riscv_common::rv32_gprs;

/// Handler for the standard RISC-V FreeRTOS layout
pub struct FreeRTOSSwitchHandlerRV32Std {
    gprs: rv32_gprs,
}

impl FreeRTOSSwitchHandlerRV32Std {
    /// Creates a new, zeroed `FreeRTOSSwitchHandlerRV32Std`
    pub fn new() -> Self {
        FreeRTOSSwitchHandlerRV32Std {
            gprs: rv32_gprs::new(),
        }
    }
}

impl freertos_switch_handler for FreeRTOSSwitchHandlerRV32Std {
    /// Pushes the cached internal state (`gprs`, `mstatus`, `pc`) back into the 
    /// physical hardware via the debugger interface.
    fn write_cur_registers(&self) -> bool {
        for i in 1..32 {
            bmp_write_register(i as u32, self.gprs.gprs[i]);
        }
        // mstatus is CSR 0x300. BMP maps CSRs at GDB offset 128.
        bmp_write_register(0x300 + 128, self.gprs.mstatus);
        bmp_write_register(32, self.gprs.pc);
        true
    }

    /// Reads the physical hardware state (`gprs`, `mstatus`, `pc`) via the debugger 
    /// interface and caches it internally.
    fn read_cur_registers(&mut self) -> bool {
        let regs = bmp_read_registers();
        if regs.len() < 33 {
            crate::bmpwarning!("Incorrect # of registers {}", regs.len());
            return false;
        }
        self.gprs.gprs[1..32].copy_from_slice(&regs[1..32]);
        self.gprs.sp = regs[2];
        self.gprs.pc = regs[32];
        
        // mstatus is CSR 0x300. BMP maps CSRs at GDB offset 128.
        if let Some(ms) = bmp_read_register(0x300 + 128) {
            self.gprs.mstatus = ms;
        } else {
            self.gprs.mstatus = 0;
        }
        true
    }

    /// Serializes the current internal cached state out to the target's memory (stack)
    /// in the exact standard RV32 layout expected by the port.
    fn write_registers_to_stack(&mut self) -> bool {
        self.gprs.sp -= 124; // 31 words
        self.gprs.pointer = self.gprs.sp;
        
        self.gprs.push32(self.gprs.pc); // mepc
        self.gprs.push(1, 2); // x1
        self.gprs.push(5, 32); // x5..x31
        self.gprs.push32(0); // xCriticalNesting
        self.gprs.push32(self.gprs.mstatus); // mstatus
        true
    }

    /// Reads the state from the target's memory (stack) at `address` and populates
    /// the internal cached state, adhering to the standard RV32 FreeRTOS stack layout.
    fn read_registers_from_addr(&mut self, address: u32) -> bool {
        // In standard RV32, stack is 31 words.
        self.gprs.pointer = address;
        self.gprs.pc = self.gprs.pop32(); // mepc (offset 0)
        self.gprs.gprs[1] = self.gprs.pop32(); // x1 (offset 4)
        self.gprs.pop(5, 32); // x5..x31 (offsets 8..112)
        
        let _critical_nesting = self.gprs.pop32(); // xCriticalNesting (offset 116)
        self.gprs.mstatus = self.gprs.pop32(); // mstatus (offset 120)
        
        self.gprs.sp = self.gprs.pointer;
        self.gprs.gprs[2] = self.gprs.sp; // Fix: Set SP in GPRs so it gets written to hardware
        true
    }

    /// Retrieves the current stack pointer
    fn get_sp(&self) -> u32 {
        self.gprs.sp
    }
}
