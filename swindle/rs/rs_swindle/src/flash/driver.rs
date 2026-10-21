/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2022 1BitSquared <info@1bitsquared.com>
 * Written by Rafael Silva <perigoso@riseup.net>
 *
 * Rust conversion of the flush operation model of
 * blackmagic/src/target/target_flash.c (target_internal.h flash_operation_e and
 * the flash driver callbacks it drives), made for the Swindle firmware. This
 * file is a translation of that C API into Rust traits and stays under the same
 * license as the original Black Magic code reproduced below.
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
