/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2023 1BitSquared <info@1bitsquared.com>
 * Written by Rachel Mant <git@dragonmux.network>
 * All rights reserved.
 *
 * Rust conversion of blackmagic/src/target/riscv32.c (riscv32_probe and the
 * RV32 target operations), made for the Swindle firmware. This file is a
 * translation of that C source into Rust and stays under the same license as
 * the original Black Magic code reproduced below.
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

//! Native Rust TargetDriver implementation for RISC-V targets.

use super::c_adapter::RiscvTargetContext;
use super::mem::RiscvMem;
use crate::target::c_adapter::{target_get_options_c, target_set_attached_c, TargetC};
use crate::target::driver::{HaltReason, TargetDriver, TargetError};

/// First-class Rust target driver for RISC-V cores.
pub struct RiscvTargetDriver {
    pub ctx: *mut RiscvTargetContext,
    pub target: *mut TargetC,
}

impl RiscvTargetDriver {
    #[inline]
    fn ctx(&self) -> &mut RiscvTargetContext {
        unsafe { &mut *self.ctx }
    }
}

impl TargetDriver for RiscvTargetDriver {
    fn mem_read(&self, dest: &mut [u8], addr: u32) -> Result<(), TargetError> {
        let ctx = self.ctx();
        RiscvMem::read(&mut ctx.hart, addr, dest)
            .map_err(|_| TargetError::AccessFault)
    }

    fn mem_write(&self, addr: u32, src: &[u8]) -> Result<(), TargetError> {
        let ctx = self.ctx();
        RiscvMem::write(&mut ctx.hart, addr, src)
            .map_err(|_| TargetError::AccessFault)
    }

    fn mem_read_nostop(&self, dest: &mut [u8], addr: u32) -> Result<bool, TargetError> {
        let ctx = self.ctx();
        RiscvMem::read_nostop(&mut ctx.hart, addr, dest)
            .map_err(|_| TargetError::AccessFault)
    }

    fn mem_write_nostop(&self, addr: u32, src: &[u8]) -> Result<bool, TargetError> {
        let ctx = self.ctx();
        RiscvMem::write_nostop(&mut ctx.hart, addr, src)
            .map_err(|_| TargetError::AccessFault)
    }

    fn needs_halt_for_mem(&self) -> bool {
        if self.target.is_null() {
            return true;
        }
        let opts = unsafe { target_get_options_c(self.target) };
        (opts & (1 << 30)) == 0
    }

    fn reg_read(&self, reg: u32, buf: &mut [u8]) -> Result<usize, TargetError> {
        let ctx = self.ctx();
        ctx.hart
            .reg_read(reg, buf)
            .map_err(|_| TargetError::AccessFault)
    }

    fn reg_write(&self, reg: u32, data: &[u8]) -> Result<usize, TargetError> {
        let ctx = self.ctx();
        ctx.hart
            .reg_write(reg, data)
            .map_err(|_| TargetError::AccessFault)
    }

    fn regs_read(&self, buf: &mut [u8]) -> Result<(), TargetError> {
        let ctx = self.ctx();
        ctx.hart
            .regs_read(buf)
            .map_err(|_| TargetError::AccessFault)
    }

    fn regs_write(&self, data: &[u8]) -> Result<(), TargetError> {
        let ctx = self.ctx();
        ctx.hart
            .regs_write(data)
            .map_err(|_| TargetError::AccessFault)
    }

    fn regs_size(&self) -> usize {
        self.ctx().hart.isa.regs_size()
    }

    fn reset(&self) {
        let ctx = self.ctx();
        let _ = ctx.hart.reset();
    }

    fn halt_request(&self) {
        let ctx = self.ctx();
        let _ = ctx.hart.halt_request();
    }

    fn halt_poll(&self) -> (HaltReason, u32) {
        let ctx = self.ctx();
        ctx.hart.halt_poll()
    }

    fn halt_resume(&self, step: bool) {
        let ctx = self.ctx();
        let _ = ctx.hart.halt_resume(step);
    }

    fn attach(&mut self) -> Result<(), TargetError> {
        let ctx = self.ctx();
        ctx.hart
            .halt_request()
            .map_err(|_| TargetError::AccessFault)?;
        if !self.target.is_null() {
            unsafe { target_set_attached_c(self.target, true) };
        }
        Ok(())
    }

    fn detach(&mut self) {
        if !self.target.is_null() {
            crate::bmp::disconnect_cleanup(self.target);
            unsafe { target_set_attached_c(self.target, false) };
        }
    }
}
