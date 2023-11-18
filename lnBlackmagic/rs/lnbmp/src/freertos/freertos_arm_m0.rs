
use crate::freertos::freertos_trait::freertos_switch_handler;
/**
 * 
 */
pub struct freertos_switch_handler_m0
{
    registers : [u32;15],
}

impl freertos_switch_handler_m0 {
    pub fn new() -> Self
    {
        freertos_switch_handler_m0 
        {
            registers : [0;15],
        }
    }
}


/**
 * 
 */
impl freertos_switch_handler for freertos_switch_handler_m0
{
    /**
     * 
     */
    fn write_current_registers(&self ) -> bool
    {
        false
    }
    /**
     * 
     */
    fn read_current_registers(&self )->bool
    {
        false
    }
    /**
     * 
     */
    fn write_registers_to_addr(&self,  address : u32) -> bool
    {
        false
    }
    /**
     * 
     */
    fn read_registers_from_addr(&self, address : u32)->bool
    {
        false
    }
}
// EOF