
/**
 * 
 */

 enum freertos_task_state
 {
    running     = b'X' as isize,
    blocked     = b'B' as isize,
    ready       = b'R' as isize,
    deleted     = b'D' as isize,
    suspended   = b'S' as isize,
 }
pub struct freertos_task_info 
{
    name         : [u8;16],
    tcb_no       : u32,
    registers    : [u32;36],
    top_of_stack : u32,
    priority     : u32,
    state        : freertos_task_state,
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
    fn get_info(&mut self, index: usize) -> Option<&freertos_task_info>
    {
        None
    }
}

//