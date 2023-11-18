crate::setup_log!(true);
use crate::{bmplog, bmpwarning};


/**
 * 
 */

 #[derive(Clone,Copy)]
 pub enum freertos_task_state
 {
    running     = b'X' as isize,
    blocked     = b'B' as isize,
    ready       = b'R' as isize,
    deleted     = b'D' as isize,
    suspended   = b'S' as isize,
 }
 
 pub struct freertos_task_info 
{
    pub name            : [u8;8],
    pub tcb_addr        : u32,
    pub tcb_no          : u32,    
    pub top_of_stack    : u32,
    pub bottom_of_stack : u32,
    pub priority        : u32,
    pub state           : freertos_task_state,
}

impl freertos_task_state
{
    pub fn as_str(&self) -> &str
    {
        match self
        {
            freertos_task_state::running => "running",
            freertos_task_state::blocked => "blocked",
            freertos_task_state::ready => "ready",
            freertos_task_state::deleted => "deleted",
            freertos_task_state::suspended => "suspended",
          // _ => "???",
        }
    }
}

impl freertos_task_info
{
    pub fn print_tcb(&self)
    {
        let name_as_string = unsafe { core::str::from_utf8_unchecked(&self.name) };
        bmplog!("TCB {} at 0x{:x} name [{}]\n", self.tcb_no, self.tcb_addr, name_as_string);
        bmplog!("\t top of stack 0x{:x}\n", self.top_of_stack);
        bmplog!("\t priority {}\n", self.priority);
        bmplog!("\t state {}\n", self.state as usize);
    }
}



/**
 * 
 */
pub trait freertos_switch_handler
{
    fn write_current_registers(&self) -> bool;
    fn read_current_registers(&mut self)->bool;
    fn write_registers_to_stack(&mut self) -> bool;
    fn read_registers_from_addr(&mut self, address : u32)->bool;
    fn get_sp(&self) -> u32;
}

//