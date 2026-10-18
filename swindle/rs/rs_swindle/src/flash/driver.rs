//! Traits and core types for flash memory operations.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FlashOperation {
    None = 0,
    Erase = 1,
    MassErase = 2,
    Write = 3,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FlashError {
    AddressOutOfBounds,
    UnalignedAccess,
    DriverFailed,
    BufferAllocationFailed,
    NotPrepared,
    NoActiveFlash,
}

/// Abstract flash driver trait implemented by hardware target drivers,
/// SPI flash controllers, or C-ABI adapters.
pub trait FlashDriver {
    /// Prepare the flash controller for an upcoming operation (erase, write, mass erase).
    fn prepare(&mut self, op: FlashOperation) -> Result<(), FlashError>;

    /// Erase an aligned block of flash memory.
    fn erase_block(&mut self, addr: u32, len: usize) -> Result<(), FlashError>;

    /// Whether this driver supports a native hardware mass erase.
    fn has_mass_erase(&self) -> bool {
        false
    }

    /// Perform a full chip / specialized mass erase.
    fn mass_erase(&mut self) -> Result<(), FlashError> {
        Err(FlashError::DriverFailed)
    }

    /// Write an aligned block of data to flash memory.
    fn write_block(&mut self, dest: u32, data: &[u8]) -> Result<(), FlashError>;

    /// Finalize flash operations and return controller to idle state.
    fn done(&mut self) -> Result<(), FlashError>;
}
