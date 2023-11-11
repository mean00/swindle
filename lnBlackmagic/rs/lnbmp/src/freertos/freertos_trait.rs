
/**
 * 
 */

pub struct freertos_info <'a>
{
    name        : &'a str,
    tcb_no      : u32,
    registers   : [u32;36],
}
/**
 * 
 */
pub trait freertos_handler
{
    fn read_tcbs(&mut self) -> bool
    {
        false
    }
    fn switch_to(&mut self, _tcb : usize ) -> bool
    {
        false
    }
    fn get_info(&mut self, index: usize) -> Option<&freertos_info>
    {
        None
    }
}
//