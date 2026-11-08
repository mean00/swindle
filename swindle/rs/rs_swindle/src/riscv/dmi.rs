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

/// Which wire the Debug Module is reached on.
///
/// The per-op RPC leaves split this into two classes (`RV.DM_READ` /
/// `RV.DM_WRITE` for RVSWD, `SDI.DM_READ` / `SDI.DM_WRITE` for WCH single-wire).
/// The bulk memory leaves ([`DmiLeg::wire_id`]) carry it as a parameter instead,
/// so one leaf serves both legs and both ends still agree on the numbering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmiLeg {
    /// The RVSWD leg (`bmp_rv_read` / `bmp_rv_write`).
    Rvswd,
    /// The WCH SDI leg (`riscv_extra::bmp_sdi_read` / `bmp_sdi_write`).
    Sdi,
}

impl DmiLeg {
    /// Wire value of the `leg` parameter of `RV.MEM_READ` / `RV.MEM_WRITE`.
    pub const fn wire_id(self) -> u8 {
        match self {
            Self::Rvswd => 0,
            Self::Sdi => 1,
        }
    }

    /// Decode a [`Self::wire_id`]; `None` for a value this build does not know.
    pub const fn from_wire_id(id: u32) -> Option<Self> {
        match id {
            0 => Some(Self::Rvswd),
            1 => Some(Self::Sdi),
            _ => None,
        }
    }
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

    /// Borrow the C `riscv_dmi_s` this transport wraps.
    ///
    /// `None` when the pointer is null, i.e. for a transport built before any
    /// scan found a DM; every method then reports a bus error or a value that
    /// makes callers back off. The individual entry points are copied out of
    /// the borrow (`Option<fn>` is `Copy`), so no reference is held across the
    /// `extern "C"` call that follows.
    fn dmi_ref(&self) -> Option<&RiscvDmiC> {
        // SAFETY: `new()` is only ever handed a live `riscv_dmi_s` (it lives in
        // the hart/TAP context, which outlives this wrapper) and the pointer is
        // only read here, never freed. A null pointer yields `None`.
        unsafe { self.dmi.as_ref() }
    }

    /// Read the DTM designer code (JEP-106).
    pub fn designer_code(&self) -> u16 {
        self.dmi_ref().map_or(0, |dmi| dmi.designer_code)
    }
}

impl DmiTransport for CDmiTransport {
    fn read(&mut self, addr: u8) -> Result<u32, DmiError> {
        let Some(dmi) = self.dmi_ref() else {
            return Err(DmiError::BusError);
        };
        let Some(read_fn) = dmi.read else {
            return Err(DmiError::NotSupported);
        };
        let mut val = 0u32;
        let ptr = core::ptr::from_ref(dmi).cast_mut();
        // SAFETY: `ptr` is the live `riscv_dmi_s` the entry point was
        // registered with (see `dmi_ref`), and `val` outlives the call.
        if unsafe { read_fn(ptr, addr as u32, &mut val) } {
            Ok(val)
        } else {
            let fault = self.dmi_ref().map_or(0, |d| d.fault);
            Err(crate::platform::wire_error::WireError::check(fault)
                .err()
                .map_or(DmiError::Fault, DmiError::from))
        }
    }

    fn write(&mut self, addr: u8, val: u32) -> Result<(), DmiError> {
        let Some(dmi) = self.dmi_ref() else {
            return Err(DmiError::BusError);
        };
        let Some(write_fn) = dmi.write else {
            return Err(DmiError::NotSupported);
        };
        let ptr = core::ptr::from_ref(dmi).cast_mut();
        // SAFETY: as in `read` - `ptr` is the live `riscv_dmi_s` for `write_fn`.
        if unsafe { write_fn(ptr, addr as u32, val) } {
            Ok(())
        } else {
            let fault = self.dmi_ref().map_or(0, |d| d.fault);
            Err(crate::platform::wire_error::WireError::check(fault)
                .err()
                .map_or(DmiError::Fault, DmiError::from))
        }
    }

    fn quiesce(&mut self) {
        let Some(quiesce_fn) = self.dmi_ref().and_then(|dmi| dmi.quiesce) else {
            return;
        };
        // SAFETY: entry point of the live `riscv_dmi_s`; it takes no DMI
        // pointer (the C side resolves the transport from its own state).
        unsafe { quiesce_fn(core::ptr::null_mut()) };
    }

    fn invalidate_caches(&mut self) {
        let Some(dmi) = self.dmi_ref() else {
            return;
        };
        let Some(invalidate_fn) = dmi.invalidate_caches else {
            return;
        };
        let ptr = core::ptr::from_ref(dmi).cast_mut();
        // SAFETY: as in `read` - `ptr` is the live `riscv_dmi_s` for `invalidate_fn`.
        unsafe { invalidate_fn(ptr) };
    }

    fn needs_read_confirm(&self) -> bool {
        self.dmi_ref().is_some_and(|dmi| dmi.read_confirm)
    }
}

#[cfg(test)]
mod tests {
    use super::DmiLeg;

    /// The `leg` byte of `RV.MEM_READ` / `RV.MEM_WRITE` is a wire value: it has
    /// to survive the round trip and stay stable, since one leaf serves both DM
    /// wires.
    #[test]
    fn dmi_leg_wire_ids_round_trip() {
        for leg in [DmiLeg::Rvswd, DmiLeg::Sdi] {
            assert_eq!(DmiLeg::from_wire_id(leg.wire_id() as u32), Some(leg));
        }
        // The numbering is part of the wire format, not an implementation detail.
        assert_eq!(DmiLeg::Rvswd.wire_id(), 0);
        assert_eq!(DmiLeg::Sdi.wire_id(), 1);
        // An unknown leg is rejected instead of silently driving the other wire.
        assert_eq!(DmiLeg::from_wire_id(2), None);
    }
}

