//! ARM Cortex-M3/M4 FreeRTOS Switch Handler
//! 
//! Implements `freertos_switch_handler` for Cortex-M3/M4 cores without FPU stack framing.
//!
//! Stack Layout (Descending):
//! ```text
//! High address
//!  |
//!  |
//!  \/ decreasing
//! +-------------------------------+  <-- PSP point before exception
//! |  xPSR                         |
//! |  Return PC (from task)        |  \
//! |  LR (R14)                     |  | Core
//! |  R12                          |  | Registers
//! |  R3,
//! |  R2
//! |  R1
//! |  R0               |  / (Auto)
//! +-------------------------------+  <-- PSP after exception
//! |  S0
//! |  S1
//! |   ...
//! |  S14
//! |  S15
//! |  FPSCR
//! +-------------------------------+
//! |  R4                          |  \
//! |  R5                          |  |
//! |  ...                         |  | Extra Registers
//! |  R11                         |  | (Saved by FreeRTOS)
//! +-------------------------------+  <-- PSP after manual save
//! ```
//! The `gpr.push` call will *FIRST* decrease the stack and then push the registers.
use crate::freertos::freertos_arm_core::freertos_cortexm_core;
use crate::freertos::freertos_trait::freertos_switch_handler;
setup_log!(false);
//use crate::bmplog;

/// EXC_RETURN value for thread mode, no FPU, using PSP.
const EXC_RETURN: u32 = 0xFFFFFFFD;

/// Context switcher for ARM Cortex-M3/M4 standard FreeRTOS ports.
pub struct freertos_switch_handler_m3 {
    gpr: freertos_cortexm_core,
}

impl freertos_switch_handler_m3 {
    /// Instantiates a new M3 context switcher.
    pub fn new() -> Self {
        freertos_switch_handler_m3 {
            gpr: freertos_cortexm_core::new(),
        }
    }
}

impl freertos_switch_handler for freertos_switch_handler_m3 {
    fn write_cur_registers(&self) -> bool {
        self.gpr.write_current_gpr_registers()
    }
    /*
     * copy actual registers to internal
     */
    fn read_cur_registers(&mut self) -> bool {
        self.gpr.read_current_gpr_registers()
    }
    /*
     * write register dump to adr, careful the register are out of order
     * We write them as if it was a freertos task switch
     */
    fn write_registers_to_stack(&mut self) -> bool {
        self.gpr.pointer = self.gpr.registers[13];
        bmplog!("Pusing registers to stack at 0x{:x}\n", self.gpr.pointer);
        
        // CM3/CM4 (No FPU) Layout:
        // push_ascending decrements pointer first, then writes.
        self.gpr.push_ascending(16, 17); // xpsr
        self.gpr.push_ascending(14, 16); // LR, PC
        self.gpr.push_ascending(12, 13); // R12
        self.gpr.push_ascending(0, 4);   // R0,R1,R2,R3
        self.gpr.push_ascending(4, 12);  // R4..R11 (lowest address = top_of_stack)

        self.gpr.registers[13] = self.gpr.pointer;
        true
    }
    /*
     * read register dump from adr, careful the register are out of order
     */
    fn read_registers_from_addr(&mut self, address: u32) -> bool {
        bmplog!("Reading registers from  0x{:x}\n", address);
        self.gpr.pointer = address;
        // CM3/CM4 (No FPU) Layout:
        self.gpr.pop_ascending(4, 12); // r4..r11
        self.gpr.pop_ascending(0, 4); // r0..r3
        self.gpr.pop_ascending(12, 13); // R12
        self.gpr.pop_ascending(14, 16); // LR/PC
        self.gpr.pop_ascending(16, 17); // XPSR
        // Set SP (R13) to the top of the saved stack frame.
        self.gpr.registers[13] = self.gpr.pointer;
        true
    }

    fn get_sp(&self) -> u32 {
        bmplog!("Reading SP =  0x{:x}\n", self.gpr.get_sp());
        self.gpr.get_sp()
    }

}
// EOF
