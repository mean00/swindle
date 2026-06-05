/*
 *  Stack layout for cortex M33 without FPU without MPU
Newer stack
    Extra
        psplim     r13 *temp
        lr         r14
        R4..r11   10 registers = 40 bytes
Interrupt
        r0--r3    8 registers = 32 regs => total 18 reg = 72 bytes
        r12
        r14 (lr)
        r15 (pc)
        r16 (xpsr)
Original stack
 */
/*


 M0..M3
 R12 PC LR XSPR

*/
use crate::freertos::freertos_arm_core::freertos_cortexm_core;
use crate::freertos::freertos_trait::freertos_switch_handler;

const STACKED_REGISTER_SIZE: u32 = 72;
const PSPLIM: usize = 14;
/*
 *
 */
pub struct freertos_switch_handler_m33 {
    gpr: freertos_cortexm_core,
}
/*
 *
 */
impl freertos_switch_handler_m33 {
    pub fn new() -> Self {
        freertos_switch_handler_m33 {
            gpr: freertos_cortexm_core::new(),
        }
    }
}

/*
 *
 */
impl freertos_switch_handler for freertos_switch_handler_m33 {
    /*
     * write internal to actual registers
     */
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
        // Push in the same order as real FreeRTOS M33 PendSV, matching the
        // layout that read_registers_from_addr() expects:
        //
        //   Low address (top_of_stack):
        //     R4..R11     (manual save, stmdb {r4-r11})
        //     R0,R1,R2,R3 (exception frame, hardware auto-stack)
        //     R12
        //     LR (R14)
        //     PC (R15)
        //     xPSR
        //     PSPLIM
        //   High address (original PSP):
        //
        // push_ascending decrements pointer first, then writes. So the
        // *last* push lands at the lowest address (top_of_stack).
        // Push PSPLIM first (highest address), then exception frame, then R4..R11.
        self.gpr.push_ascending(PSPLIM, PSPLIM + 1); // psplim (highest address)
        self.gpr.push_ascending(16, 17); // xpsr
        self.gpr.push_ascending(14, 16); // LR, PC
        self.gpr.push_ascending(12, 13); // R12
        self.gpr.push_ascending(0, 4);   // R0,R1,R2,R3
        self.gpr.push_ascending(4, 12);  // R4..R11 (lowest address = top_of_stack)
        // FPU IF NEEDED TODO
        // Update SP to the final pointer value (lowest address = first pushed register)
        self.gpr.registers[13] = self.gpr.pointer;

        true
    }
    /*
     * read register dump from adr, careful the register are out of order
     */
    fn read_registers_from_addr(&mut self, address: u32) -> bool {
        self.gpr.pointer = address;
        // Read in the same order as write_registers_to_stack (matching real FreeRTOS)
        self.gpr.pop_ascending(4, 12); // r4..r11
        self.gpr.pop_ascending(0, 4); // r0..r3
        self.gpr.pop_ascending(12, 13); // R12
        self.gpr.pop_ascending(14, 16); // LR/PC
        self.gpr.pop_ascending(16, 17); // XPSR
        self.gpr.pop_ascending(PSPLIM, PSPLIM + 1); // psplim
        self.gpr.registers[13] = self.gpr.pointer;
        true
    }

    fn get_sp(&self) -> u32 {
        self.gpr.get_sp()
    }

}
// EOF
