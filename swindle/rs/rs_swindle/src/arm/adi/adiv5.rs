/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2015 Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2018-2021 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (C) 2022-2024 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the transport-generic ADIv5 memory access and control routines
 * of blackmagic/src/target/adiv5.c, made for the Swindle firmware. This
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

//! ADIv5 DebugPort and AccessPort engine.

use crate::target::TargetError;
use super::regs::*;
use super::swd::{HardwareSwd, SwdTransport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdiError {
    Fault,
    Timeout,
    ProtocolError,
    NullPointer,
    NotConfigured,
    Unsupported,
}

impl From<AdiError> for TargetError {
    fn from(err: AdiError) -> Self {
        match err {
            AdiError::Fault => TargetError::AccessFault,
            AdiError::Timeout => TargetError::Timeout,
            AdiError::Unsupported => TargetError::Unsupported,
            _ => TargetError::InternalError,
        }
    }
}

/// Logical representation of an ADIv5 Debug Port.
#[repr(C)]
pub struct DebugPort<T: SwdTransport = HardwareSwd> {
    pub transport: T,
    pub version: u8,
    pub designer_code: u16,
    pub partno: u16,
    pub target_designer_code: u16,
    pub target_partno: u16,
    pub fault: u8,
    pub select_cache: u32,
    pub select_cached: bool,
    pub select1_cache: u32,
    pub select1_cached: bool,
    pub dev_index: u8,
    pub targetsel: u32,
    pub refcnt: u32,
}

impl<T: SwdTransport> DebugPort<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            version: 0,
            designer_code: 0,
            partno: 0,
            target_designer_code: 0,
            target_partno: 0,
            fault: 0,
            select_cache: 0,
            select_cached: false,
            select1_cache: 0,
            select1_cached: false,
            dev_index: 0,
            targetsel: 0,
            refcnt: 0,
        }
    }

    pub fn invalidate_caches(&mut self) {
        self.select_cached = false;
        self.select1_cached = false;
    }

    /// Check if a wire-level or sticky fault occurred.
    pub fn check_fault(&mut self) -> Result<(), AdiError> {
        let hw_fault = crate::platform::swd::fault() as u8;
        if hw_fault != 0 {
            self.fault = hw_fault;
            return Err(AdiError::Fault);
        }
        if self.fault != 0 {
            return Err(AdiError::Fault);
        }
        Ok(())
    }

    /// Import the platform's sticky wire fault and hand it to the caller.
    ///
    /// Entry points that touch the wire through [`SwdTransport::raw_access`]
    /// bypass [`Self::check_fault`], so the latch has to be pulled in explicitly
    /// here — otherwise faults are reported as 0 while the wire faulted.
    pub fn wire_fault(&mut self) -> u32 {
        let hw_fault = crate::platform::swd::fault() as u8;
        if hw_fault != 0 {
            self.fault = hw_fault;
        }
        self.fault as u32
    }

    pub fn select_bank(&mut self, apsel: u8, apbank: u8, dpbank: u8) -> Result<(), AdiError> {
        let select = ((apsel as u32) << ADIV5_DP_SELECT_APSEL_OFFSET)
            | ((apbank as u32) << ADIV5_DP_SELECT_APBANK_OFFSET)
            | ((dpbank as u32) << ADIV5_DP_SELECT_DPBANK_OFFSET);

        if !self.select_cached || self.select_cache != select {
            self.transport.dp_write(ADIV5_DP_SELECT, select);
            self.select_cache = select;
            self.select_cached = true;
            self.check_fault()?;
        }
        Ok(())
    }

    /// Write a DP register and keep cached DP_SELECT in sync.
    pub fn dp_write(&mut self, addr: u16, value: u32) -> Result<(), AdiError> {
        if addr == ADIV5_DP_SELECT {
            self.select_cache = value;
            self.select_cached = true;
        }
        self.transport.dp_write(addr, value);
        self.check_fault()
    }

    /// Read a DP register with fault checking.
    pub fn dp_read(&mut self, addr: u16) -> Result<u32, AdiError> {
        let val = self.transport.dp_read(addr);
        self.check_fault()?;
        Ok(val)
    }

    /// Read an AP register with bank switching.
    pub fn ap_read(&mut self, apsel: u8, addr: u16) -> Result<u32, AdiError> {
        self.ap_reg_read(apsel, 0, addr)
    }

    /// Write an AP register with bank switching.
    pub fn ap_write(&mut self, apsel: u8, addr: u16, value: u32) -> Result<(), AdiError> {
        self.ap_reg_write(apsel, 0, addr, value)
    }

    /// Select AP register bank supporting both ADIv5 (apsel) and ADIv6 (ap_address).
    pub fn select_ap_bank(&mut self, apsel: u8, ap_address: u32, addr: u16) -> Result<(), AdiError> {
        if self.version >= 3 {
            let select = ap_address | ((addr & ADIV6_AP_BANK_MASK) as u32);
            if !self.select_cached || self.select_cache != select {
                self.transport.dp_write(ADIV5_DP_SELECT, select);
                self.select_cache = select;
                self.select_cached = true;
                self.check_fault()?;
            }
        } else {
            let apbank = ((addr >> 4) & 0x0f) as u8;
            self.select_bank(apsel, apbank, 0)?;
        }
        Ok(())
    }

    /// Read an AP register from a specific AP (supporting ADIv5 and ADIv6).
    pub fn ap_reg_read(&mut self, apsel: u8, ap_address: u32, addr: u16) -> Result<u32, AdiError> {
        self.select_ap_bank(apsel, ap_address, addr)?;
        let val = self.transport.dp_read(addr);
        self.check_fault()?;
        Ok(val)
    }

    /// Write an AP register to a specific AP (supporting ADIv5 and ADIv6).
    pub fn ap_reg_write(&mut self, apsel: u8, ap_address: u32, addr: u16, value: u32) -> Result<(), AdiError> {
        self.select_ap_bank(apsel, ap_address, addr)?;
        self.transport.dp_write(addr, value);
        self.check_fault()
    }

    /// Read DPIDR register without error checks where possible.
    pub fn read_dpidr(&mut self) -> u32 {
        let val = self.transport.read_no_check(ADIV5_DP_DPIDR);
        if val != 0 {
            return val;
        }
        self.transport.raw_access(ADIV5_LOW_READ, ADIV5_DP_DPIDR, 0)
    }

    /// Power cycle the APs: request power down, then power up.
    pub fn power_cycle_aps(&mut self) -> Result<(), AdiError> {
        self.select_bank(0, 0, 0)?;
        self.transport.dp_write(ADIV5_DP_CTRLSTAT, 0);

        let ack = ADIV5_DP_CTRLSTAT_CSYSPWRUPACK | ADIV5_DP_CTRLSTAT_CDBGPWRUPACK;
        for _ in 0..250 {
            let status = self.transport.dp_read(ADIV5_DP_CTRLSTAT);
            if (status & ack) == 0 {
                break;
            }
        }

        let req = ADIV5_DP_CTRLSTAT_CSYSPWRUPREQ | ADIV5_DP_CTRLSTAT_CDBGPWRUPREQ;
        self.transport.dp_write(ADIV5_DP_CTRLSTAT, req);

        for _ in 0..250 {
            let status = self.transport.dp_read(ADIV5_DP_CTRLSTAT);
            if (status & ack) == ack {
                return Ok(());
            }
        }
        Err(AdiError::Timeout)
    }

    /// Handle and clear sticky errors and protocol faults.
    pub fn error(&mut self, protocol_recovery: bool) -> u32 {
        let hw_fault = crate::platform::swd::fault() as u8;
        if hw_fault != 0 {
            self.fault = hw_fault;
        }
        if (self.version >= 2 && self.fault != 0) || protocol_recovery {
            self.invalidate_caches();
            crate::arm::adi::swd::swd_line_reset_sequence(&mut self.transport, true);
            if self.version >= 2 {
                self.transport.write_no_check(ADIV5_DP_TARGETSEL, self.targetsel);
            }
            self.transport.raw_access(ADIV5_LOW_READ, ADIV5_DP_DPIDR, 0);
        }

        let err = self.transport.read_no_check(ADIV5_DP_CTRLSTAT);
        if err == 0 {
            if !protocol_recovery {
                return self.error(true);
            }
            return ADIV5_DP_CTRLSTAT_ERRORS;
        }

        let mut clr = 0;
        if (err & ADIV5_DP_CTRLSTAT_STICKYORUN) != 0 {
            clr |= ADIV5_DP_ABORT_ORUNERRCLR;
        }
        if (err & ADIV5_DP_CTRLSTAT_STICKYCMP) != 0 {
            clr |= ADIV5_DP_ABORT_STKCMPCLR;
        }
        if (err & ADIV5_DP_CTRLSTAT_STICKYERR) != 0 {
            clr |= ADIV5_DP_ABORT_STKERRCLR;
        }
        if (err & ADIV5_DP_CTRLSTAT_WDATAERR) != 0 {
            clr |= ADIV5_DP_ABORT_WDERRCLR;
        }

        if clr != 0 {
            self.transport.write_no_check(ADIV5_DP_ABORT, clr);
        }
        self.fault = 0;
        crate::platform::swd::clear_fault();

        err & ADIV5_DP_CTRLSTAT_ERRORS
    }

    /// Clear all sticky errors.
    pub fn clear_errors(&mut self) {
        self.error(false);
    }
}

/// Logical representation of an ADIv5 Access Port.
#[repr(C)]
pub struct AccessPort {
    pub dp: *mut DebugPort<HardwareSwd>,
    pub apsel: u8,
    pub idr: u32,
    pub base: u32,
    pub csw: u32,
    pub csw_cache: u32,
    pub csw_cached: bool,
    pub designer_code: u16,
    pub partno: u16,
    pub ap_cortexm_demcr: u32,
    pub flags: u32,
    pub refcnt: u32,
    pub ap_address: u32,
}

impl AccessPort {
    pub fn new(dp: *mut DebugPort<HardwareSwd>, apsel: u8) -> Self {
        if !dp.is_null() {
            unsafe {
                (*dp).refcnt += 1;
            }
        }
        Self {
            dp,
            apsel,
            idr: 0,
            base: 0,
            csw: ADIV5_AP_CSW_DBGSWENABLE
                | ADIV5_AP_CSW_PROT_DEFAULT,
            csw_cache: 0,
            csw_cached: false,
            designer_code: 0,
            partno: 0,
            ap_cortexm_demcr: 0,
            flags: 0,
            refcnt: 1,
            ap_address: 0,
        }
    }

    pub fn new_adiv6(dp: *mut DebugPort<HardwareSwd>, ap_address: u32) -> Self {
        if !dp.is_null() {
            unsafe {
                (*dp).refcnt += 1;
            }
        }
        Self {
            dp,
            apsel: 0,
            idr: 0,
            base: 0,
            csw: ADIV5_AP_CSW_DBGSWENABLE
                | ADIV5_AP_CSW_PROT_DEFAULT,
            csw_cache: 0,
            csw_cached: false,
            designer_code: 0,
            partno: 0,
            ap_cortexm_demcr: 0,
            flags: 0,
            refcnt: 1,
            ap_address,
        }
    }

    pub fn ap_read(&mut self, addr: u16) -> Result<u32, AdiError> {
        if self.dp.is_null() {
            return Err(AdiError::NullPointer);
        }
        let dp = unsafe { &mut *self.dp };
        dp.ap_reg_read(self.apsel, self.ap_address, addr)
    }

    pub fn ap_write(&mut self, addr: u16, value: u32) -> Result<(), AdiError> {
        if self.dp.is_null() {
            return Err(AdiError::NullPointer);
        }
        let dp = unsafe { &mut *self.dp };
        dp.ap_reg_write(self.apsel, self.ap_address, addr, value)?;
        if addr == ADIV5_AP_CSW {
            self.csw_cache = value;
            self.csw_cached = true;
        }
        Ok(())
    }

    pub fn invalidate_caches(&mut self) {
        self.csw_cached = false;
    }

    /// Configure this AP by reading its IDR, CSW, CFG, and BASE.
    pub fn configure(&mut self) -> Result<bool, AdiError> {
        if self.dp.is_null() {
            return Err(AdiError::NullPointer);
        }
        self.idr = self.ap_read(ADIV5_AP_IDR)?;
        if self.idr == 0 {
            return Ok(false);
        }

        let ap_class = ((self.idr & ADIV5_AP_IDR_CLASS_MASK) >> ADIV5_AP_IDR_CLASS_OFFSET) as u8;
        let ap_type = ((self.idr & ADIV5_AP_IDR_TYPE_MASK) >> ADIV5_AP_IDR_TYPE_OFFSET) as u8;

        if ap_class == ADIV5_AP_IDR_CLASS_MEM && (1..=8).contains(&ap_type) {
            match ap_type {
                ARM_AP_TYPE_AHB3 | ARM_AP_TYPE_AHB5 | ARM_AP_TYPE_AHB5_HPROT => {
                    self.csw &= !ADIV5_AP_CSW_AHB_HPROT_MASK;
                    self.csw |= ADIV5_AP_CSW_AHB_MASTERTYPE
                        | ADIV5_AP_CSW_AHB_HPROT_DATA
                        | ADIV5_AP_CSW_AHB_HPROT_PRIV;
                    if (self.csw & ADIV5_AP_CSW_SPIDEN) != 0 {
                        self.csw &= !ADIV5_AP_CSW_AHB_HNONSEC;
                    } else {
                        self.csw |= ADIV5_AP_CSW_AHB_HNONSEC;
                    }
                }
                ARM_AP_TYPE_APB4_5 => {
                    self.csw |= ADIV5_AP_CSW_AHB_HPROT_PRIV;
                }
                _ => {}
            }

            self.base = self.ap_read(ADIV5_AP_BASE)? & 0xffff_f000;
            let _cfg = self.ap_read(ADIV5_AP_CFG)?;

            if (self.csw & ADIV5_AP_CSW_TRINPROG) != 0 {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Set up CSW and TAR for memory accesses.
    pub fn setup_mem_access(&mut self, addr: u32, align_size: u32) -> Result<(), AdiError> {
        if self.dp.is_null() {
            return Err(AdiError::NullPointer);
        }
        let mut csw = (self.csw & !ADIV5_AP_CSW_SIZE_MASK) | ADIV5_AP_CSW_ADDRINC_SINGLE;
        match align_size {
            ALIGN_8BIT => csw |= ADIV5_AP_CSW_SIZE_BYTE,
            ALIGN_16BIT => csw |= ADIV5_AP_CSW_SIZE_HALFWORD,
            _ => csw |= ADIV5_AP_CSW_SIZE_WORD,
        }
        if !self.csw_cached || self.csw_cache != csw {
            self.ap_write(ADIV5_AP_CSW, csw)?;
        }
        self.ap_write(ADIV5_AP_TAR, addr)
    }

    /// Setup banked debug register access (for Cortex-M register file transfers).
    pub fn debug_access_setup(&mut self) -> Result<(), AdiError> {
        if self.dp.is_null() {
            return Err(AdiError::NullPointer);
        }
        self.setup_mem_access(CORTEXM_DHCSR_ADDR, ADIV5_AP_CSW_SIZE_WORD)?;
        let dp = unsafe { &mut *self.dp };
        if dp.version >= 3 {
            let select = self.ap_address | (ADIV5_AP_BD0 & ADIV6_AP_BANK_MASK) as u32;
            if !dp.select_cached || dp.select_cache != select {
                dp.transport.dp_write(ADIV5_DP_SELECT, select);
                dp.select_cache = select;
                dp.select_cached = true;
                dp.check_fault()?;
            }
        } else {
            dp.select_bank(self.apsel, 1, 0)?;
        }
        Ok(())
    }

    /// Memory read bytes with alignment and 1024-byte auto-increment boundary management.
    pub fn mem_read_bytes(&mut self, dest: &mut [u8], src: u32) -> Result<(), AdiError> {
        if dest.is_empty() {
            return Ok(());
        }
        if self.dp.is_null() {
            return Err(AdiError::NullPointer);
        }

        let dp = unsafe { &mut *self.dp };
        let align = if (src & 3) == 0 && (dest.len() & 3) == 0 {
            ALIGN_32BIT
        } else if (src & 1) == 0 && (dest.len() & 1) == 0 {
            ALIGN_16BIT
        } else {
            ALIGN_8BIT
        };

        let stride = 1usize << align;
        let mut cur_addr = src;
        let mut idx = 0;

        self.setup_mem_access(cur_addr, align)?;

        while idx < dest.len() {
            if cur_addr != src && (cur_addr & 0x3ff) == 0 {
                self.ap_write(ADIV5_AP_TAR, cur_addr)?;
            }

            let val = dp.transport.dp_read(ADIV5_AP_DRW);
            dp.check_fault()?;

            match align {
                ALIGN_8BIT => {
                    dest[idx] = ((val >> (8 * (cur_addr & 3))) & 0xff) as u8;
                }
                ALIGN_16BIT => {
                    let hw = ((val >> (8 * (cur_addr & 2))) & 0xffff) as u16;
                    dest[idx..idx + 2].copy_from_slice(&hw.to_le_bytes());
                }
                _ => {
                    dest[idx..idx + 4].copy_from_slice(&val.to_le_bytes());
                }
            }

            idx += stride;
            cur_addr = cur_addr.wrapping_add(stride as u32);
        }
        Ok(())
    }

    /// Memory write bytes with alignment and 1024-byte boundary management.
    pub fn mem_write_bytes(&mut self, dest: u32, src: &[u8]) -> Result<(), AdiError> {
        self.mem_write_aligned(dest, src, ALIGN_32BIT)
    }

    /// Memory write with explicit alignment (8-bit, 16-bit, 32-bit).
    pub fn mem_write_aligned(&mut self, dest: u32, src: &[u8], _align_hint: u32) -> Result<(), AdiError> {
        if src.is_empty() {
            return Ok(());
        }
        if self.dp.is_null() {
            return Err(AdiError::NullPointer);
        }

        let dp = unsafe { &mut *self.dp };
        let align = if (dest & 3) == 0 && (src.len() & 3) == 0 {
            ALIGN_32BIT
        } else if (dest & 1) == 0 && (src.len() & 1) == 0 {
            ALIGN_16BIT
        } else {
            ALIGN_8BIT
        };

        let stride = 1usize << align;
        let mut cur_addr = dest;
        let mut idx = 0;

        self.setup_mem_access(cur_addr, align)?;

        while idx < src.len() {
            dp.check_fault()?;
            if cur_addr != dest && (cur_addr & 0x3ff) == 0 {
                self.ap_write(ADIV5_AP_TAR, cur_addr)?;
            }

            let val = match align {
                ALIGN_8BIT => (src[idx] as u32) << (8 * (cur_addr & 3)),
                ALIGN_16BIT => {
                    let hw = u16::from_le_bytes([src[idx], src[idx + 1]]) as u32;
                    hw << (8 * (cur_addr & 2))
                }
                _ => u32::from_le_bytes([src[idx], src[idx + 1], src[idx + 2], src[idx + 3]]),
            };

            dp.transport.dp_write(ADIV5_AP_DRW, val);

            idx += stride;
            cur_addr = cur_addr.wrapping_add(stride as u32);
        }

        // Dummy read to complete write pipeline
        dp.transport.dp_read(ADIV5_DP_RDBUFF);
        dp.check_fault()
    }

    /// Read a single 32-bit word from target memory.
    pub fn mem_read32(&mut self, addr: u32) -> Result<u32, AdiError> {
        let mut bytes = [0u8; 4];
        self.mem_read_bytes(&mut bytes, addr)?;
        Ok(u32::from_le_bytes(bytes))
    }

    /// Write a single 32-bit word to target memory.
    pub fn mem_write32(&mut self, addr: u32, val: u32) -> Result<(), AdiError> {
        self.mem_write_bytes(addr, &val.to_le_bytes())
    }
}
