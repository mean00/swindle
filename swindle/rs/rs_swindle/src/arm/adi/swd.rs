/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011 Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2020-2021 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (C) 2022-2024 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the SWD-specific functions of the ARM Debug Interface
 * of blackmagic/src/target/adiv5_swd.c, made for the Swindle firmware. This
 * file is derived from that C source and stays under the same license as the original
 * Black Magic code reproduced below.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

//! Low-level SWD transport trait and primitives.

use crate::platform::wire_error::{WireError, WireResult};
use super::regs::*;

/// Physical SWD line interface trait.
pub trait SwdTransport {
    /// Perform a raw read or write of a DP or AP register.
    /// `addr`: A 16-bit register address (with bit 8 indicating AP vs DP).
    /// `rnw`: `ADIV5_LOW_READ` or `ADIV5_LOW_WRITE`.
    /// `value`: Value to write (or 0 for reads).
    /// Returns the read value (or written value) and sets sticky fault status if applicable.
    fn raw_access(&mut self, rnw: u8, addr: u16, value: u32) -> u32;

    /// Fast write to an address without fault checking.
    fn write_no_check(&mut self, addr: u16, data: u32) -> bool;

    /// Fast read from an address without fault checking.
    fn read_no_check(&mut self, addr: u16) -> u32;

    /// Perform a raw read or write returning an explicit [`WireResult<u32>`].
    fn try_raw_access(&mut self, rnw: u8, addr: u16, value: u32) -> WireResult<u32> {
        let val = self.raw_access(rnw, addr, value);
        let fault = crate::platform::swd::fault() as u8;
        if fault == 0 {
            Ok(val)
        } else {
            WireError::into_read_result(fault, val)
        }
    }

    /// Fast write to an address returning an explicit [`WireResult<()>`].
    fn try_write_no_check(&mut self, addr: u16, data: u32) -> WireResult<()> {
        if self.write_no_check(addr, data) {
            Err(WireError::Fault)
        } else {
            Ok(())
        }
    }

    /// Fast read from an address returning an explicit [`WireResult<u32>`].
    fn try_read_no_check(&mut self, addr: u16) -> WireResult<u32> {
        let val = self.read_no_check(addr);
        if val == 0 {
            Err(WireError::NoResponse)
        } else {
            Ok(val)
        }
    }

    /// Read a DP or AP register.
    /// If `addr & ADIV5_APnDP != 0` (AP register), does posted read followed by RDBUFF read.
    /// If `addr & ADIV5_APnDP == 0` (DP register), returns raw read directly.
    fn dp_read(&mut self, addr: u16) -> u32 {
        if (addr & ADIV5_APnDP) != 0 {
            self.raw_access(ADIV5_LOW_READ, addr, 0);
            self.raw_access(ADIV5_LOW_READ, ADIV5_DP_RDBUFF, 0)
        } else {
            self.raw_access(ADIV5_LOW_READ, addr, 0)
        }
    }

    /// Write a DP or AP register.
    fn dp_write(&mut self, addr: u16, value: u32) {
        self.raw_access(ADIV5_LOW_WRITE, addr, value);
    }

    /// Raw bit-level SWD sequence output.
    fn raw_swd_write(&mut self, tick: u32, value: u32);

    /// Clear DP error / fault flags by writing ABORT.
    fn abort(&mut self, flags: u32) {
        self.raw_access(ADIV5_LOW_WRITE, ADIV5_DP_ABORT, flags);
    }
}

/// Global hardware SWD implementation calling into Swindle platform hooks.
pub struct HardwareSwd;

impl SwdTransport for HardwareSwd {
    fn raw_access(&mut self, rnw: u8, addr: u16, value: u32) -> u32 {
        crate::platform::swd::raw_access(rnw, addr, value)
    }

    fn write_no_check(&mut self, addr: u16, data: u32) -> bool {
        crate::platform::swd::write_no_check(addr, data)
    }

    fn read_no_check(&mut self, addr: u16) -> u32 {
        crate::platform::swd::read_no_check(addr)
    }

    fn try_raw_access(&mut self, rnw: u8, addr: u16, value: u32) -> WireResult<u32> {
        crate::platform::swd::try_raw_access(rnw, addr, value)
    }

    fn try_write_no_check(&mut self, addr: u16, data: u32) -> WireResult<()> {
        crate::platform::swd::try_write_no_check(addr, data)
    }

    fn try_read_no_check(&mut self, addr: u16) -> WireResult<u32> {
        crate::platform::swd::try_read_no_check(addr)
    }

    fn raw_swd_write(&mut self, tick: u32, value: u32) {
        crate::platform::swd::raw_write(tick, value);
    }
}

/// Reset SWD line with >= 50 high cycles followed by optional idle cycles.
pub fn swd_line_reset_sequence<T: SwdTransport>(transport: &mut T, idle_cycles: bool) {
    transport.raw_swd_write(32, 0xffff_ffff);
    transport.raw_swd_write(if idle_cycles { 32 } else { 28 }, 0x0fff_ffff);
}

/// Switch out of dormant state into SWD.
pub fn dormant_to_swd_sequence<T: SwdTransport>(transport: &mut T) {
    swd_line_reset_sequence(transport, false);
    transport.raw_swd_write(5, ADIV5_JTAG_TO_DORMANT_SEQUENCE0);
    transport.raw_swd_write(31, ADIV5_JTAG_TO_DORMANT_SEQUENCE1);
    transport.raw_swd_write(8, ADIV5_JTAG_TO_DORMANT_SEQUENCE2);

    transport.raw_swd_write(32, ADIV5_SELECTION_ALERT_SEQUENCE_0);
    transport.raw_swd_write(32, ADIV5_SELECTION_ALERT_SEQUENCE_1);
    transport.raw_swd_write(32, ADIV5_SELECTION_ALERT_SEQUENCE_2);
    transport.raw_swd_write(32, ADIV5_SELECTION_ALERT_SEQUENCE_3);

    transport.raw_swd_write(12, ADIV5_ACTIVATION_CODE_ARM_SWD_DP << 4);
    swd_line_reset_sequence(transport, true);
}

/// Deprecated JTAG-to-SWD select sequence.
pub fn jtag_to_swd_sequence<T: SwdTransport>(transport: &mut T) {
    swd_line_reset_sequence(transport, false);
    transport.raw_swd_write(16, ADIV5_JTAG_TO_SWD_SELECT_SEQUENCE);
    swd_line_reset_sequence(transport, true);
}
