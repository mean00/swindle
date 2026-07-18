//! ARM Cortex-M4F FreeRTOS Switch Handler
//! 
//! Implements `freertos_switch_handler` for Cortex-M4F cores with FPU stack framing.
//!
//! Stack Layout (Descending):
//! ```text
//! High address
//!  |
//!  |
//!  \/ decreasing
//! +-------------------------------+  <-- PSP point before exception
//! |  FPSCR                        |
//! |  Reserved                     |
//! |  S15..S0                      |  \
//! |  xPSR                         |  | Core
//! |  Return PC (from task)        |  | Registers
//! |  LR (R14)                     |  |
//! |  R12                          |  |
//! |  R3,                          |  |
//! |  R2                           |  |
//! |  R1                           |  |
//! |  R0                           |  / (Auto-stacked)
//! +-------------------------------+  <-- PSP after exception
//! |  S31..S16                     |  \
//! |  EXC_RETURN                   |  | Extra Registers
//! |  R11..R4                      |  | (Saved manually by PendSV)
//! +-------------------------------+  <-- PSP after manual save
//! ```
use crate::freertos::freertos_arm_core::freertos_cortexm_core;
use crate::freertos::freertos_trait::freertos_switch_handler;
use crate::bmp::{bmp_read_mem32};
setup_log!(false);

/// EXC_RETURN value for thread mode, using PSP, without FPU.
const EXC_RETURN_NO_FPU: u32 = 0xFFFFFFFD;

/// Context switcher for ARM Cortex-M4F standard FreeRTOS ports.
pub struct freertos_switch_handler_m4f {
    gpr: freertos_cortexm_core,
}

impl freertos_switch_handler_m4f {
    pub fn new() -> Self {
        freertos_switch_handler_m4f {
            gpr: freertos_cortexm_core::new(),
        }
    }
}

impl freertos_switch_handler for freertos_switch_handler_m4f {
    fn write_cur_registers(&self) -> bool {
        self.gpr.write_current_gpr_registers()
    }
    
    fn read_cur_registers(&mut self) -> bool {
        self.gpr.read_current_gpr_registers()
    }
    
    fn write_registers_to_stack(&mut self) -> bool {
        self.gpr.pointer = self.gpr.registers[13];
        bmplog!("Pusing registers to stack at 0x{:x}\n", self.gpr.pointer);
        
        // Push the core exception frame
        self.gpr.push_ascending(16, 17); // xpsr
        self.gpr.push_ascending(14, 16); // LR, PC
        self.gpr.push_ascending(12, 13); // R12
        self.gpr.push_ascending(0, 4);   // R0,R1,R2,R3
        
        // We do not push actual FPU registers (S16-S31) here to save time and space,
        // so we must use an EXC_RETURN value that indicates no FPU context was saved manually.
        // Bit 4 = 1 means no FPU context was pushed manually.
        self.gpr.write_ascending(EXC_RETURN_NO_FPU);
        
        // Push the remaining core registers
        self.gpr.push_ascending(4, 12);  // R4..R11 (lowest address = top_of_stack)

        self.gpr.registers[13] = self.gpr.pointer;
        true
    }
    
    fn read_registers_from_addr(&mut self, address: u32) -> bool {
        bmplog!("Reading registers from  0x{:x}\n", address);
        self.gpr.pointer = address;
        
        self.gpr.pop_ascending(4, 12); // r4..r11
        
        // Read EXC_RETURN word
        let mut exc_buf = [0u32; 1];
        if !bmp_read_mem32(self.gpr.pointer, &mut exc_buf) {
            return false;
        }
        let exc_return = exc_buf[0];
        self.gpr.pointer += 4; // Skip EXC_RETURN

        // Check if FPU was used (bit 4 == 0)
        if (exc_return & 0x10) == 0 {
            // S16-S31 were pushed manually (16 words = 64 bytes)
            // We skip them to reach the core exception frame
            self.gpr.pointer += 64;
        }

        self.gpr.pop_ascending(0, 4); // r0..r3
        self.gpr.pop_ascending(12, 13); // R12
        self.gpr.pop_ascending(14, 16); // LR/PC
        self.gpr.pop_ascending(16, 17); // XPSR
        
        self.gpr.registers[13] = self.gpr.pointer;
        true
    }

    fn get_sp(&self) -> u32 {
        self.gpr.get_sp()
    }
}
