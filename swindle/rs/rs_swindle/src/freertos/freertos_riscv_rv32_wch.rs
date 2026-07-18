/// \brief WCH/Bumblebee RISC-V FreeRTOS support
///
/// This module implements the FreeRTOS context switch handling for the WCH / Bumblebee 
/// RISC-V RV32 layout (such as the CH32V30x series). 
///
/// Stack layout for WCH RV32:
/// - PC (mepc)
/// - mstatus
/// - [FPU registers if FPU dirty bit is set]
/// - ra/x1
/// - x5..x31

use crate::bmp::{bmp_read_registers, bmp_write_register, bmp_read_register};
use crate::freertos::freertos_trait::freertos_switch_handler;
use crate::freertos::freertos_symbols::LAYOUT_CH32_FPU;
use super::freertos_riscv_common::{rv32_gprs, get_current_layout, STACKED_REGISTER_SIZE};

/// Handler for the WCH / Bumblebee RISC-V FreeRTOS layout
pub struct FreeRTOSSwitchHandlerCH32 {
    gprs: rv32_gprs,
}

impl FreeRTOSSwitchHandlerCH32 {
    /// Creates a new, zeroed `FreeRTOSSwitchHandlerCH32`
    pub fn new() -> Self {
        FreeRTOSSwitchHandlerCH32 {
            gprs: rv32_gprs::new(),
        }
    }
}

impl freertos_switch_handler for FreeRTOSSwitchHandlerCH32 {
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
    /// in the exact layout expected by the WCH/Bumblebee OS port.
    fn write_registers_to_stack(&mut self) -> bool {
        // For write, we only provide a basic integer stack frame for now.
        // We'll write the frame without FPU dirty flag set.
        self.gprs.sp -= STACKED_REGISTER_SIZE as u32; // adjust stack to be at the beginning
        self.gprs.pointer = self.gprs.sp;

        self.gprs.push32(self.gprs.pc);
        self.gprs.push32(self.gprs.mstatus & !(3 << 13)); // clear FS bits to indicate FPU not dirty

        self.gprs.push(1, 2); // x1
        self.gprs.push(5, 32); // x5..x31
        true
    }

    /// Reads the state from the target's memory (stack) at `address` and populates
    /// the internal cached state, adhering to the WCH specific layout (including
    /// checking `mstatus` for FPU dirty bits).
    fn read_registers_from_addr(&mut self, address: u32) -> bool {
        let layout = get_current_layout();
        self.gprs.pointer = address;
        self.gprs.pc = self.gprs.pop32(); // PC
        self.gprs.mstatus = self.gprs.pop32(); // mstatus
        
        if layout == LAYOUT_CH32_FPU {
            // Check if FPU is dirty (bit 14 of mstatus)
            if (self.gprs.mstatus & (1 << 14)) != 0 {
                self.gprs.pointer += 128; // Skip FPU registers (32 * 4)
            }
        }
        
        self.gprs.gprs[1] = self.gprs.pop32(); // ra/x1
        self.gprs.pop(5, 32); // x5..x31
        
        self.gprs.sp = self.gprs.pointer;
        self.gprs.gprs[2] = self.gprs.sp; // Fix: Set SP in GPRs so it gets written to hardware
        true
    }

    /// Retrieves the current stack pointer
    fn get_sp(&self) -> u32 {
        self.gprs.sp
    }
}
