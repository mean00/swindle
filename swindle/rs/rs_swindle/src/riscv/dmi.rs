/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2023-2024 1BitSquared <info@1bitsquared.com>
 * Written by Rachel Mant <git@dragonmux.network>
 * All rights reserved.
 *
 * Rust conversion of blackmagic/src/target/riscv_debug.c, made for the Swindle
 * firmware. This file is a translation of that C source into Rust and stays
 * under the same license as the original Black Magic code reproduced below.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice, this
 *    list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * 3. Neither the name of the copyright holder nor the names of its
 *    contributors may be used to endorse or promote products derived from
 *    this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

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
