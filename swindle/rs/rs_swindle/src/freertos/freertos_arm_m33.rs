/*
 *  Stack layout for cortex M33 without FPU without MPU
 *
 *  Real FreeRTOS M33 PendSV (non-MPU) does:
 *    mrs r0, psp
 *    mrs r2, psplim
 *    mov r3, lr          // EXC_RETURN
 *    stmdb r0!, {r2-r11} // push PSPLIM, EXC_RETURN, R4-R11
 *    str r0, [r1]        // TCB[0] = pxTopOfStack
 *
 *  pxTopOfStack → [PSPLIM]     (r2)
 *                 [EXC_RETURN]  (r3 = 0xFFFFFFFx)
 *                 [R4]
 *                 [R5]
 *                 [R6]
 *                 [R7]
 *                 [R8]
 *                 [R9]
 *                 [R10]
 *                 [R11]
 *                 [R0]         ← exception frame (hardware auto-pushed)
 *                 [R1]
 *                 [R2]
 *                 [R3]
 *                 [R12]
 *                 [task LR]
 *                 [task PC]
 *                 [xPSR]
 */
use crate::freertos::freertos_arm_core::freertos_cortexm_core;
use crate::freertos::freertos_trait::freertos_switch_handler;
setup_log!(false);
//use crate::bmplog;

/// EXC_RETURN value for thread mode, no FPU, using PSP.
const EXC_RETURN: u32 = 0xFFFFFFFD;

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
        bmplog!("Pusing registers to stack at 0x{:x}\n", self.gpr.pointer);
        // Push in the same order as real FreeRTOS M33 PendSV, matching the
        // layout that read_registers_from_addr() expects:
        //
        //   Low address (top_of_stack):
        //     PSPLIM      (r2, saved by stmdb)
        //     EXC_RETURN  (r3, saved by stmdb)
        //     R4..R11     (manual save, stmdb {r2-r11})
        //     R0,R1,R2,R3 (exception frame, hardware auto-stack)
        //     R12
        //     LR (R14)
        //     PC (R15)
        //     xPSR
        //   High address (original PSP):
        //
        // push_ascending decrements pointer first, then writes. So the
        // *last* push lands at the lowest address (top_of_stack).
        // Push exception frame first (higher addresses), then R4..R11, then PSPLIM/EXC_RETURN.
        self.gpr.push_ascending(16, 17); // xpsr
        self.gpr.push_ascending(14, 16); // LR, PC
        self.gpr.push_ascending(12, 13); // R12
        self.gpr.push_ascending(0, 4);   // R0,R1,R2,R3
        self.gpr.push_ascending(4, 12);  // R4..R11
        // EXC_RETURN: real PendSV saves r3 (EXC_RETURN value) after PSPLIM
        self.gpr.write_ascending(EXC_RETURN);
        // PSPLIM: real PendSV saves r2 (PSPLIM) as the first word
        // We don't know the real PSPLIM value, store 0 (no MPU)
        self.gpr.write_ascending(0);
        // Update SP to the final pointer value (lowest address = first pushed register)
        self.gpr.registers[13] = self.gpr.pointer;

        true
    }
    /*
     * read register dump from adr, careful the register are out of order
     */
    fn read_registers_from_addr(&mut self, address: u32) -> bool {
        bmplog!("Reading registers from  0x{:x}\n", address);
        self.gpr.pointer = address;
        // Read in the same order as write_registers_to_stack (matching real FreeRTOS)
        // Skip PSPLIM and EXC_RETURN (first 2 words = 8 bytes)
        self.gpr.pointer += 8;
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