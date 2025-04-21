/*
*   Just bounce to M3
*/
use crate::freertos::freertos_arm_m3::freertos_switch_handler_m3 as cm3;
use crate::freertos::freertos_trait::freertos_switch_handler;

/*
 *
 */
pub struct freertos_switch_handler_m0 {
    parent: cm3,
}
/*
 *
 */
impl freertos_switch_handler_m0 {
    pub fn new() -> Self {
        freertos_switch_handler_m0 { parent: cm3::new() }
    }
}
/*
 *
 */
impl freertos_switch_handler for freertos_switch_handler_m0 {
    /*
     * write internal to actual registers
     */
    fn write_current_registers(&self) -> bool {
        self.parent.write_current_registers()
    }
    /*
     * copy actual registers to internal
     */
    fn read_current_registers(&mut self) -> bool {
        self.parent.read_current_registers()
    }
    /*
     * write register dump to adr, careful the register are out of order
     * We write them as if it was a freertos task switch
     */
    fn write_registers_to_stack(&mut self) -> bool {
        self.parent.write_registers_to_stack()
    }
    /*
     * read register dump from adr, careful the register are out of order
     */
    fn read_registers_from_addr(&mut self, address: u32) -> bool {
        self.parent.read_registers_from_addr(address)
    }

    fn get_sp(&self) -> u32 {
        self.parent.get_sp()
    }
}
// EOF
