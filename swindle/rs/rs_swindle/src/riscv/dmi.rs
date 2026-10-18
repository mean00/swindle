//! DMI (Debug Module Interface) transport abstraction.
//!
//! Provides a zero-allocation trait [`DmiTransport`] and concrete implementations:
//! - [`CDmiTransport`]: bridges to C `riscv_dmi_s` function pointers (SDI, RVSWD, ADI).
//! - Direct Rust drivers where applicable.

use core::ffi::c_void;

/// Errors arising from DMI bus transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmiError {
    /// Target returned a busy / retry condition.
    Busy,
    /// Target returned a transaction failure / fault.
    Fault,
    /// Bus timeout while waiting for completion.
    Timeout,
    /// Physical bus or transport error.
    BusError,
    /// Feature or command not supported by target.
    NotSupported,
}

/// Abstract Debug Module Interface transport.
pub trait DmiTransport {
    /// Read a 32-bit DMI register at `addr`.
    fn read(&mut self, addr: u8) -> Result<u32, DmiError>;

    /// Write a 32-bit DMI register at `addr`.
    fn write(&mut self, addr: u8, val: u32) -> Result<(), DmiError>;

    /// Quiesce transport pipeline after streaming burst (BMP 100).
    fn quiesce(&mut self) {}

    /// Invalidate any transport write caches after reset (BMP 83).
    fn invalidate_caches(&mut self) {}

    /// True if the physical link cannot report transfer loss (e.g. SDI 1-wire)
    /// and reads require two-pass confirmation (BMP 103).
    fn needs_read_confirm(&self) -> bool {
        false
    }
}

/// C `struct riscv_dmi` representation for FFI interop.
#[repr(C)]
pub struct RiscvDmiC {
    pub ref_count: u32,
    pub designer_code: u16,
    pub version: u32,
    pub dev_index: u8,
    pub idle_cycles: u8,
    pub address_width: u8,
    pub fault: u8,
    pub read_confirm: bool,

    pub prepare: Option<unsafe extern "C" fn(target: *mut c_void)>,
    pub quiesce: Option<unsafe extern "C" fn(target: *mut c_void)>,
    pub read: Option<unsafe extern "C" fn(dmi: *mut RiscvDmiC, address: u32, value: *mut u32) -> bool>,
    pub write: Option<unsafe extern "C" fn(dmi: *mut RiscvDmiC, address: u32, value: u32) -> bool>,
    pub invalidate_caches: Option<unsafe extern "C" fn(dmi: *mut RiscvDmiC)>,
}

/// DMI transport wrapper around an existing C `riscv_dmi_s`.
pub struct CDmiTransport {
    dmi: *mut RiscvDmiC,
}

impl CDmiTransport {
    /// Create a new transport wrapper from a C `riscv_dmi_s` pointer.
    ///
    /// # Safety
    /// `dmi` must point to a valid, live `riscv_dmi_s` structure.
    pub unsafe fn new(dmi: *mut RiscvDmiC) -> Self {
        Self { dmi }
    }

    /// Access the underlying C DMI pointer.
    pub fn as_ptr(&self) -> *mut RiscvDmiC {
        self.dmi
    }

    /// Read the DTM designer code (JEP-106).
    pub fn designer_code(&self) -> u16 {
        if self.dmi.is_null() {
            0
        } else {
            unsafe { (*self.dmi).designer_code }
        }
    }
}

impl DmiTransport for CDmiTransport {
    fn read(&mut self, addr: u8) -> Result<u32, DmiError> {
        if self.dmi.is_null() {
            return Err(DmiError::BusError);
        }
        unsafe {
            if let Some(read_fn) = (*self.dmi).read {
                let mut val = 0u32;
                if read_fn(self.dmi, addr as u32, &mut val) {
                    Ok(val)
                } else {
                    Err(DmiError::Fault)
                }
            } else {
                Err(DmiError::NotSupported)
            }
        }
    }

    fn write(&mut self, addr: u8, val: u32) -> Result<(), DmiError> {
        if self.dmi.is_null() {
            return Err(DmiError::BusError);
        }
        unsafe {
            if let Some(write_fn) = (*self.dmi).write {
                if write_fn(self.dmi, addr as u32, val) {
                    Ok(())
                } else {
                    Err(DmiError::Fault)
                }
            } else {
                Err(DmiError::NotSupported)
            }
        }
    }

    fn quiesce(&mut self) {
        if !self.dmi.is_null() {
            unsafe {
                if let Some(q_fn) = (*self.dmi).quiesce {
                    q_fn(core::ptr::null_mut());
                }
            }
        }
    }

    fn invalidate_caches(&mut self) {
        if !self.dmi.is_null() {
            unsafe {
                if let Some(inv_fn) = (*self.dmi).invalidate_caches {
                    inv_fn(self.dmi);
                }
            }
        }
    }

    fn needs_read_confirm(&self) -> bool {
        if self.dmi.is_null() {
            false
        } else {
            unsafe { (*self.dmi).read_confirm }
        }
    }
}
