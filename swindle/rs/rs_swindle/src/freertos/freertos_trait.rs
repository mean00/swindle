//! FreeRTOS Core Traits and Types
//! 
//! Defines the core `freertos_switch_handler` interface that all architecture-specific
//! context switchers must implement, along with structures to represent task states
//! and basic Task Control Block (TCB) information extracted from memory.

setup_log!(false);
//use crate::bmplog;

/// Represents the FreeRTOS scheduling state of a task.
#[derive(Clone, Copy)]
pub enum freertos_task_state {
    running = b'X' as isize,
    blocked = b'B' as isize,
    ready = b'R' as isize,
    deleted = b'D' as isize,
    suspended = b'S' as isize,
}

/// A parsed snapshot of a FreeRTOS Task Control Block (TCB).
#[derive(Clone)]
pub struct freertos_task_info {
    /// 8-byte ASCII name of the task
    pub name: [u8; 8],
    /// Address of the TCB in target memory
    pub tcb_addr: u32,
    /// Task index number
    pub tcb_no: u32,
    /// Top of stack address (where context is saved)
    pub top_of_stack: u32,
    /// Bottom of stack address
    pub bottom_of_stack: u32,
    /// FreeRTOS priority level
    pub priority: u32,
    /// Discovered execution state
    pub state: freertos_task_state,
}

impl freertos_task_state {
    pub fn as_str(&self) -> &str {
        match self {
            freertos_task_state::running => "running",
            freertos_task_state::blocked => "blocked",
            freertos_task_state::ready => "ready",
            freertos_task_state::deleted => "deleted",
            freertos_task_state::suspended => "suspended",
            // _ => "???",
        }
    }
}

impl freertos_task_info {
    pub fn print_tcb(&self) {
        let name_as_string = unsafe { core::str::from_utf8_unchecked(&self.name) };
        bmplog!(
            "TCB {} at 0x{:x} name [{}]\n",
            self.tcb_no,
            self.tcb_addr,
            name_as_string
        );
        bmplog!("\t top of stack 0x{:x}\n", self.top_of_stack);
        bmplog!("\t priority {}\n", self.priority);
        bmplog!("\t state {}\n", self.state.as_str());
    }
}

/// Defines the interface for an architecture-specific context switcher.
/// 
/// This trait is implemented by architecture backends (e.g., M3, M33, RV32) to
/// orchestrate moving register state between the physical CPU and the target memory
/// stack in a format compatible with the FreeRTOS port's `PendSV` or equivalent handler.
pub trait freertos_switch_handler {
    /// Writes the cached internal registers to the actual CPU hardware.
    fn write_cur_registers(&self) -> bool;
    
    /// Reads the actual CPU hardware registers into the internal cache.
    fn read_cur_registers(&mut self) -> bool;
    
    /// Pushes the cached internal registers to the task's stack memory 
    /// (simulating a context save).
    fn write_registers_to_stack(&mut self) -> bool;
    
    /// Pops the registers from the task's stack memory at `address` into 
    /// the internal cache (simulating a context restore).
    fn read_registers_from_addr(&mut self, address: u32) -> bool;
    
    /// Retrieves the current logical stack pointer.
    fn get_sp(&self) -> u32;
}

//
