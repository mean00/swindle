//! Memory access dispatch (Sysbus, Abstractauto streaming, Progbuf, and SDI retry).

use super::dm::*;
use super::dmi::{DmiError, DmiTransport};
use super::hart::RiscvHart;
use crate::target::driver::TargetError;

// Sysbus Control Bits
pub const RV_SBCS_SBACCESS_8: u32 = 0 << 17;
pub const RV_SBCS_SBACCESS_16: u32 = 1 << 17;
pub const RV_SBCS_SBACCESS_32: u32 = 2 << 17;
pub const RV_SBCS_SBAUTOINCREMENT: u32 = 1 << 16;
pub const RV_SBCS_SBREADONDATA: u32 = 1 << 15;
pub const RV_SBCS_SBREADONADDR: u32 = 1 << 20;
pub const RV_SBCS_SBBUSY: u32 = 1 << 21;
pub const RV_SBCS_SBBUSYERROR: u32 = 1 << 22;
pub const RV_SBCS_SBERROR_MASK: u32 = 0x7 << 12;

/// Memory access dispatcher for RISC-V targets.
pub struct RiscvMem;

impl RiscvMem {
    /// Read arbitrary memory bytes from `addr` into `dest`.
    pub fn read(
        hart: &RiscvHart,
        dmi: &mut dyn DmiTransport,
        addr: u32,
        dest: &mut [u8],
    ) -> Result<(), TargetError> {
        if dest.is_empty() {
            return Ok(());
        }

        // Bounded retry loop for SDI read confirmation (patch 102 & 103)
        let retry_count = if dmi.needs_read_confirm() { 4 } else { 1 };
        for _ in 0..retry_count {
            if Self::read_attempt(hart, dmi, addr, dest).is_ok() {
                return Ok(());
            }
        }
        Err(TargetError::AccessFault)
    }

    fn read_attempt(
        hart: &RiscvHart,
        dmi: &mut dyn DmiTransport,
        addr: u32,
        dest: &mut [u8],
    ) -> Result<(), TargetError> {
        if hart.has_sysbus {
            Self::sysbus_read(dmi, addr, dest)
        } else {
            Self::abstract_read(hart, dmi, addr, dest)
        }
    }

    /// Write arbitrary memory bytes from `src` to `addr`.
    pub fn write(
        hart: &RiscvHart,
        dmi: &mut dyn DmiTransport,
        addr: u32,
        src: &[u8],
    ) -> Result<(), TargetError> {
        if src.is_empty() {
            return Ok(());
        }
        if hart.has_sysbus {
            Self::sysbus_write(dmi, addr, src)
        } else {
            Self::abstract_write(hart, dmi, addr, src)
        }
    }

    /// Non-halting read via Sysbus (returns true if performed).
    pub fn read_nostop(
        hart: &RiscvHart,
        dmi: &mut dyn DmiTransport,
        addr: u32,
        dest: &mut [u8],
    ) -> Result<bool, TargetError> {
        if hart.has_sysbus {
            Self::read(hart, dmi, addr, dest).map(|_| true)
        } else {
            Ok(false)
        }
    }

    /// Non-halting write via Sysbus (returns true if performed).
    pub fn write_nostop(
        hart: &RiscvHart,
        dmi: &mut dyn DmiTransport,
        addr: u32,
        src: &[u8],
    ) -> Result<bool, TargetError> {
        if hart.has_sysbus {
            Self::write(hart, dmi, addr, src).map(|_| true)
        } else {
            Ok(false)
        }
    }

    // --- Sysbus Operations ---

    fn wait_sysbus_idle(dmi: &mut dyn DmiTransport) -> Result<(), DmiError> {
        for _ in 0..1000 {
            let cs = dmi.read(RV_DM_SBCS)?;
            if (cs & (RV_SBCS_SBBUSY | RV_SBCS_SBBUSYERROR)) == 0 {
                let err = (cs & RV_SBCS_SBERROR_MASK) >> 12;
                if err != 0 {
                    // Clear error by writing 1 to the error bits
                    let _ = dmi.write(RV_DM_SBCS, cs);
                    return Err(DmiError::Fault);
                }
                return Ok(());
            }
        }
        Err(DmiError::Timeout)
    }

    fn sysbus_read(dmi: &mut dyn DmiTransport, mut addr: u32, mut dest: &mut [u8]) -> Result<(), TargetError> {
        // Word-aligned chunk loop
        while dest.len() >= 4 && (addr & 3) == 0 {
            let sbcs = RV_SBCS_SBACCESS_32 | RV_SBCS_SBAUTOINCREMENT | RV_SBCS_SBREADONADDR;
            dmi.write(RV_DM_SBCS, sbcs).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBADDRESS0, addr).map_err(|_| TargetError::AccessFault)?;
            Self::wait_sysbus_idle(dmi).map_err(|_| TargetError::AccessFault)?;

            let val = dmi.read(RV_DM_SBDATA0).map_err(|_| TargetError::AccessFault)?;
            dest[..4].copy_from_slice(&val.to_le_bytes());
            addr += 4;
            dest = &mut dest[4..];
        }

        // Halfword-aligned loop (crucial for flash programming)
        while dest.len() >= 2 && (addr & 1) == 0 {
            let sbcs = RV_SBCS_SBACCESS_16 | RV_SBCS_SBREADONADDR;
            dmi.write(RV_DM_SBCS, sbcs).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBADDRESS0, addr).map_err(|_| TargetError::AccessFault)?;
            Self::wait_sysbus_idle(dmi).map_err(|_| TargetError::AccessFault)?;

            let val = dmi.read(RV_DM_SBDATA0).map_err(|_| TargetError::AccessFault)? as u16;
            dest[..2].copy_from_slice(&val.to_le_bytes());
            addr += 2;
            dest = &mut dest[2..];
        }

        // Byte-wise remainder
        while !dest.is_empty() {
            let sbcs = RV_SBCS_SBACCESS_8 | RV_SBCS_SBREADONADDR;
            dmi.write(RV_DM_SBCS, sbcs).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBADDRESS0, addr).map_err(|_| TargetError::AccessFault)?;
            Self::wait_sysbus_idle(dmi).map_err(|_| TargetError::AccessFault)?;

            let val = dmi.read(RV_DM_SBDATA0).map_err(|_| TargetError::AccessFault)? as u8;
            dest[0] = val;
            addr += 1;
            dest = &mut dest[1..];
        }

        Ok(())
    }

    fn sysbus_write(dmi: &mut dyn DmiTransport, mut addr: u32, mut src: &[u8]) -> Result<(), TargetError> {
        // Word-aligned chunk loop
        while src.len() >= 4 && (addr & 3) == 0 {
            let val = u32::from_le_bytes(src[..4].try_into().unwrap());
            let sbcs = RV_SBCS_SBACCESS_32 | RV_SBCS_SBAUTOINCREMENT;
            dmi.write(RV_DM_SBCS, sbcs).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBADDRESS0, addr).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBDATA0, val).map_err(|_| TargetError::AccessFault)?;
            Self::wait_sysbus_idle(dmi).map_err(|_| TargetError::AccessFault)?;

            addr += 4;
            src = &src[4..];
        }

        // Halfword-aligned loop (crucial for flash programming)
        while src.len() >= 2 && (addr & 1) == 0 {
            let val = u16::from_le_bytes(src[..2].try_into().unwrap()) as u32;
            let sbcs = RV_SBCS_SBACCESS_16;
            dmi.write(RV_DM_SBCS, sbcs).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBADDRESS0, addr).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBDATA0, val).map_err(|_| TargetError::AccessFault)?;
            Self::wait_sysbus_idle(dmi).map_err(|_| TargetError::AccessFault)?;

            addr += 2;
            src = &src[2..];
        }

        // Byte-wise remainder
        while !src.is_empty() {
            let val = src[0] as u32;
            let sbcs = RV_SBCS_SBACCESS_8;
            dmi.write(RV_DM_SBCS, sbcs).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBADDRESS0, addr).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_SBDATA0, val).map_err(|_| TargetError::AccessFault)?;
            Self::wait_sysbus_idle(dmi).map_err(|_| TargetError::AccessFault)?;

            addr += 1;
            src = &src[1..];
        }

        Ok(())
    }

    // --- Abstract Memory Operations ---

    fn abstract_read(
        _hart: &RiscvHart,
        dmi: &mut dyn DmiTransport,
        mut addr: u32,
        mut dest: &mut [u8],
    ) -> Result<(), TargetError> {
        // For abstract command memory access, cmdtype = 2 (access memory)
        // cmd = (2 << 24) | (aarsize << 20) | (aampostinc << 19)
        while dest.len() >= 4 && (addr & 3) == 0 {
            dmi.write(RV_DM_DATA1, addr).map_err(|_| TargetError::AccessFault)?;
            let cmd = (2 << 24) | (2 << 20) | (1 << 19); // 32-bit, postincrement
            exec_command(dmi, cmd).map_err(|_| TargetError::AccessFault)?;
            let val = dmi.read(RV_DM_DATA0).map_err(|_| TargetError::AccessFault)?;
            dest[..4].copy_from_slice(&val.to_le_bytes());
            addr += 4;
            dest = &mut dest[4..];
        }

        while dest.len() >= 2 && (addr & 1) == 0 {
            dmi.write(RV_DM_DATA1, addr).map_err(|_| TargetError::AccessFault)?;
            let cmd = (2 << 24) | (1 << 20) | (1 << 19); // 16-bit, postincrement
            exec_command(dmi, cmd).map_err(|_| TargetError::AccessFault)?;
            let val = dmi.read(RV_DM_DATA0).map_err(|_| TargetError::AccessFault)? as u16;
            dest[..2].copy_from_slice(&val.to_le_bytes());
            addr += 2;
            dest = &mut dest[2..];
        }

        while !dest.is_empty() {
            dmi.write(RV_DM_DATA1, addr).map_err(|_| TargetError::AccessFault)?;
            let cmd = (2 << 24) | (0 << 20) | (1 << 19); // 8-bit, postincrement
            exec_command(dmi, cmd).map_err(|_| TargetError::AccessFault)?;
            let val = dmi.read(RV_DM_DATA0).map_err(|_| TargetError::AccessFault)? as u8;
            dest[0] = val;
            addr += 1;
            dest = &mut dest[1..];
        }

        dmi.quiesce();
        Ok(())
    }

    fn abstract_write(
        _hart: &RiscvHart,
        dmi: &mut dyn DmiTransport,
        mut addr: u32,
        mut src: &[u8],
    ) -> Result<(), TargetError> {
        while src.len() >= 4 && (addr & 3) == 0 {
            let val = u32::from_le_bytes(src[..4].try_into().unwrap());
            dmi.write(RV_DM_DATA0, val).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_DATA1, addr).map_err(|_| TargetError::AccessFault)?;
            let cmd = (2 << 24) | (1 << 16) | (2 << 20) | (1 << 19); // write, 32-bit, postincrement
            exec_command(dmi, cmd).map_err(|_| TargetError::AccessFault)?;
            addr += 4;
            src = &src[4..];
        }

        while src.len() >= 2 && (addr & 1) == 0 {
            let val = u16::from_le_bytes(src[..2].try_into().unwrap()) as u32;
            dmi.write(RV_DM_DATA0, val).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_DATA1, addr).map_err(|_| TargetError::AccessFault)?;
            let cmd = (2 << 24) | (1 << 16) | (1 << 20) | (1 << 19); // write, 16-bit, postincrement
            exec_command(dmi, cmd).map_err(|_| TargetError::AccessFault)?;
            addr += 2;
            src = &src[2..];
        }

        while !src.is_empty() {
            let val = src[0] as u32;
            dmi.write(RV_DM_DATA0, val).map_err(|_| TargetError::AccessFault)?;
            dmi.write(RV_DM_DATA1, addr).map_err(|_| TargetError::AccessFault)?;
            let cmd = (2 << 24) | (1 << 16) | (0 << 20) | (1 << 19); // write, 8-bit, postincrement
            exec_command(dmi, cmd).map_err(|_| TargetError::AccessFault)?;
            addr += 1;
            src = &src[1..];
        }

        dmi.quiesce();
        Ok(())
    }
}
