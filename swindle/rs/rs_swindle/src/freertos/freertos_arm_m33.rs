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
        self.gpr.registers[13] -= STACKED_REGISTER_SIZE; // adjust stack to be at the beginning
        self.gpr.pointer = self.gpr.registers[13];

        self.gpr.push(14, 17); // r14/r15/R16 (xPSR)
        self.gpr.push(12, 13); // R12
        self.gpr.push(0, 4); // push  R0 to R4 excluded
        self.gpr.push(4, 12); // push  R4..r12 excluded

        self.gpr.push(14, 15); // LR
        self.gpr.push(PSPLIM, PSPLIM + 1); // psplim

        true
    }
    /*
     * read register dump from adr, careful the register are out of order
     */
    fn read_registers_from_addr(&mut self, address: u32) -> bool {
        self.gpr.registers[13] = address + STACKED_REGISTER_SIZE;
        // rewind by 16 *4=64 bytes, we dont save SP on SP but on TCP
        self.gpr.pointer = address;
        // now read the registers onto the stack
        self.gpr.pop(PSPLIM, PSPLIM + 1); // psplim
        self.gpr.pop(14, 15); // LR

        self.gpr.pop(4, 12); // push  R4..r12 excluded
        self.gpr.pop(0, 4); //
        self.gpr.pop(12, 13); // R12
        self.gpr.pop(14, 17); // r14/r15/R16 PSR

        true
    }

    fn get_sp(&self) -> u32 {
        self.gpr.get_sp()
    }
}
// EOF
