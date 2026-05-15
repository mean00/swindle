/*
High address
 |
 |
 \/ decreasing
+-------------------------------+  <-- PSP point before exception
|  xPSR                         |
|  Return PC (from task)        |  \
|  LR (R14)                     |  | Core
|  R12                          |  | Registers
|  R3,
|  R2
|  R1
|  R0               |  / (Auto)
+-------------------------------+  <-- PSP after exception
|  S0
|  S1
|   ...
|  S14
|  S15
|  FPSCR
+-------------------------------+
|  R4                          |  \
|  R5                          |  |
|  ...                         |  | Extra Registers
|  R11                         |  | (Saved by FreeRTOS)
+-------------------------------+  <-- PSP after manual save

The gpr.push call will *FIRST* decrease the stack and then push the registers

*/
use crate::freertos::freertos_arm_core::freertos_cortexm_core;
use crate::freertos::freertos_trait::freertos_switch_handler;
crate::setup_log!(false);
//use crate::bmplog;

/*
 *
 */
pub struct freertos_switch_handler_m3 {
    gpr: freertos_cortexm_core,
}
/*
 *
 */
impl freertos_switch_handler_m3 {
    pub fn new() -> Self {
        freertos_switch_handler_m3 {
            gpr: freertos_cortexm_core::new(),
        }
    }
}

/*
 *
 */
impl freertos_switch_handler for freertos_switch_handler_m3 {
    /*
     * write internal to actual registers
     */
    fn write_current_registers(&self) -> bool {
        self.gpr.write_current_gpr_registers()
    }
    /*
     * copy actual registers to internal
     */
    fn read_current_registers(&mut self) -> bool {
        self.gpr.read_current_gpr_registers()
    }
    /*
     * write register dump to adr, careful the register are out of order
     * We write them as if it was a freertos task switch
     */
    fn write_registers_to_stack(&mut self) -> bool {
        self.gpr.pointer = self.gpr.registers[13];
        bmplog!("Pusing registers to stack at 0x{:x}\n", self.gpr.pointer);
        // Push in the same order as real FreeRTOS PendSV:
        // First the manual-save block (R4..R11), then the exception frame (R0..R3,R12,LR,PC,xPSR)
        // Real FreeRTOS: stmdb r0!, {r4-r11}  -> R4 at lowest address, R11 at highest
        self.gpr.push_ascending(4, 12); // r4..r11 (R4 at lowest addr, matching stmdb {r4-r11})
        // Exception frame (hardware auto-stack): R0,R1,R2,R3,R12,LR,PC,xPSR
        self.gpr.push_ascending(0, 4); // r0..r1..r2.r3
        self.gpr.push_ascending(12, 13); // r12
        self.gpr.push_ascending(14, 16); // r14/r15 (LR/PC)
        self.gpr.push_ascending(16, 17); // xpsr
        // FPU IF NEEDED TODO
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
        // Read registers from the stack in the same order as they were written
        // (matching real FreeRTOS: R4..R11 first, then exception frame)
        self.gpr.pop_ascending(4, 12); // r4..r11
        // FPU TODO
        self.gpr.pop_ascending(0, 4); // r0..r3
        self.gpr.pop_ascending(12, 13); // R12
        self.gpr.pop_ascending(14, 16); // LR/PC
        self.gpr.pop_ascending(16, 17); // XPSR
        true
    }

    fn get_sp(&self) -> u32 {
        bmplog!("Reading SP =  0x{:x}\n", self.gpr.get_sp());
        self.gpr.get_sp()
    }
}
// EOF
