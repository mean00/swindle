//! Target driver trait and core execution / control types.

use super::breakwatch::Breakwatch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum HaltReason {
    Running = 0,
    Error = 1,
    Request = 2,
    Stepping = 3,
    Breakpoint = 4,
    Watchpoint = 5,
    Fault = 6,
    Unknown = 7,
}

impl From<u32> for HaltReason {
    fn from(val: u32) -> Self {
        match val {
            0 => HaltReason::Running,
            1 => HaltReason::Error,
            2 => HaltReason::Request,
            3 => HaltReason::Stepping,
            4 => HaltReason::Breakpoint,
            5 => HaltReason::Watchpoint,
            6 => HaltReason::Fault,
            _ => HaltReason::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BreakwatchType {
    BreakSoft = 0,
    BreakHard = 1,
    WatchWrite = 2,
    WatchRead = 3,
    WatchAccess = 4,
}

impl BreakwatchType {
    /// Decode the numeric tag GDB uses in its `Z`/`z` packets (C `target_breakwatch_e`).
    pub const fn from_int(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::BreakSoft),
            1 => Some(Self::BreakHard),
            2 => Some(Self::WatchWrite),
            3 => Some(Self::WatchRead),
            4 => Some(Self::WatchAccess),
            _ => None,
        }
    }

    /// True for the types a hardware trigger unit can implement.
    pub const fn is_hardware(self) -> bool {
        !matches!(self, Self::BreakSoft)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetError {
    NotAttached,
    AccessFault,
    Timeout,
    Unsupported,
    InternalError,
    /// The resource the request needs (e.g. a hardware comparator) is exhausted.
    NoResources,
    /// The entry a clear request names is not armed.
    NotFound,
}

/// Abstract driver interface for target CPU operations.
pub trait TargetDriver {
    /// Read memory from target at given 32-bit address.
    fn mem_read(&self, dest: &mut [u8], addr: u32) -> Result<(), TargetError>;

    /// Write memory to target at given 32-bit address.
    fn mem_write(&self, addr: u32, src: &[u8]) -> Result<(), TargetError>;

    /// Attempt memory read without stopping target CPU.
    fn mem_read_nostop(&self, dest: &mut [u8], addr: u32) -> Result<bool, TargetError>;

    /// Attempt memory write without stopping target CPU.
    fn mem_write_nostop(&self, addr: u32, src: &[u8]) -> Result<bool, TargetError>;

    /// Query whether memory access requires halting the CPU.
    fn needs_halt_for_mem(&self) -> bool;

    /// Read a single register by index. Returns number of bytes read.
    fn reg_read(&self, reg: u32, buf: &mut [u8]) -> Result<usize, TargetError>;

    /// Write a single register by index. Returns number of bytes written.
    fn reg_write(&self, reg: u32, data: &[u8]) -> Result<usize, TargetError>;

    /// Read all target general registers into buffer.
    fn regs_read(&self, buf: &mut [u8]) -> Result<(), TargetError>;

    /// Write all target general registers from buffer.
    fn regs_write(&self, data: &[u8]) -> Result<(), TargetError>;

    /// Total size in bytes of general registers.
    fn regs_size(&self) -> usize;

    /// Reset target CPU.
    fn reset(&self);

    /// Request halt of target CPU.
    fn halt_request(&self);

    /// Poll target execution state and return halt reason and optional watchpoint address.
    fn halt_poll(&self) -> (HaltReason, u32);

    /// Resume execution of target CPU. If `step` is true, perform a single step.
    fn halt_resume(&self, step: bool);

    /// Attach to target core.
    fn attach(&mut self) -> Result<(), TargetError>;

    /// Detach from target core.
    fn detach(&mut self);

    /// Arm a hardware breakpoint/watchpoint.
    ///
    /// The driver owns the bookkeeping: it allocates the comparator slot, records
    /// the entry in its own table and programs the unit, then reports the slot in
    /// `breakwatch.slot()`. Targets without a trigger unit keep the default.
    fn breakwatch_set(&mut self, breakwatch: &mut Breakwatch) -> Result<(), TargetError> {
        let _ = breakwatch;
        Err(TargetError::Unsupported)
    }

    /// Disarm a trigger previously armed through [`TargetDriver::breakwatch_set`],
    /// freeing its comparator slot. `breakwatch` carries the identity of the entry.
    fn breakwatch_clear(&mut self, breakwatch: &Breakwatch) -> Result<(), TargetError> {
        let _ = breakwatch;
        Err(TargetError::Unsupported)
    }

    /// Drop every armed trigger and release its comparator (detach path).
    fn breakwatch_clear_all(&mut self) {}
}
