/**
 * 
 */
pub struct freertos_cortex_m0;
/**
 * 
 */
impl freertos_handler for freertos_cortex_m0
{
    /**
     * 
     */
    fn read_tcbs(&mut self) -> bool
    {
        false
    }
    /**
     * 
     */
    fn switch_to(&mut self, _tcb : usize ) -> bool
    {
        false
    }
    /**
     * 
     */
    fn get_info_size(&mut self) -> usize
    {
        0
    }
    /**
     * 
     */
    fn get_info(&mut self, index: usize) -> Option<&freertos_info>
    {
        None
    }
}
// EOF