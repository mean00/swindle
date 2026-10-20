//! RISC-V Hart representation, register file, and run control (RV32E, RV32I, RV32IF).

use super::dm::*;
use super::dmi::{CDmiTransport, DmiError, DmiTransport};
use crate::target::driver::{HaltReason, TargetError};

// --- CSR Constants ---
pub const RV_CSR_FFLAGS: u16 = 0x001;
pub const RV_CSR_FRM: u16 = 0x002;
pub const RV_CSR_FCSR: u16 = 0x003;
pub const RV_CSR_MSTATUS: u16 = 0x300;
pub const RV_CSR_MISA: u16 = 0x301;
pub const RV_CSR_DCSR: u16 = 0x7b0;
pub const RV_CSR_DPC: u16 = 0x7b1;
pub const RV_CSR_MVENDORID: u16 = 0xf11;
pub const RV_CSR_MARCHID: u16 = 0xf12;
pub const RV_CSR_MIMPID: u16 = 0xf13;
pub const RV_CSR_MHARTID: u16 = 0xf14;

// Register bases in abstract register space
pub const RV_REG_GPR_BASE: u16 = 0x1000;
pub const RV_REG_FPR_BASE: u16 = 0x1020;

// DCSR bits
pub const RV_DCSR_STEP: u32 = 1 << 2;
pub const RV_DCSR_CAUSE_SHIFT: u32 = 6;
pub const RV_DCSR_CAUSE_MASK: u32 = 0x7 << 6;
pub const RV_DCSR_EBREAKM: u32 = 1 << 15;

// MISA bits
pub const RV_MISA_EXT_E: u32 = 1 << 4;
pub const RV_MISA_EXT_F: u32 = 1 << 5;
pub const RV_MISA_EXT_I: u32 = 1 << 8;

// GDB offsets
pub const RV_FPU_GDB_OFFSET: u32 = 33;
pub const RV_FPU_GDB_CSR_OFFSET: u32 = 66;
pub const RV_CSR_GDB_OFFSET: u32 = 896;

/// Supported RISC-V 32-bit core architectures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HartIsa {
    /// RV32E: 16 GPRs + PC (68 bytes register file).
    Rv32e,
    /// RV32I: 32 GPRs + PC (132 bytes register file).
    Rv32i,
    /// RV32I + Single-precision FPU: 32 GPRs + PC + 32 FPRs + FPU CSRs.
    Rv32iSingleFpu,
}

impl HartIsa {
    /// Number of general purpose registers (16 for RV32E, 32 for RV32I/IF).
    pub fn gpr_count(self) -> usize {
        match self {
            HartIsa::Rv32e => 16,
            HartIsa::Rv32i | HartIsa::Rv32iSingleFpu => 32,
        }
    }

    /// GDB register index of the program counter (16 for RV32E, 32 for RV32I/IF).
    pub fn pc_reg_num(self) -> u32 {
        self.gpr_count() as u32
    }

    /// Total size in bytes of bulk general registers (`regs_read`/`regs_write`).
    pub fn regs_size(self) -> usize {
        (self.gpr_count() + 1) * 4
    }

    /// True if single-precision floating point extension ('F') is supported.
    pub fn has_single_fpu(self) -> bool {
        matches!(self, HartIsa::Rv32iSingleFpu)
    }
}

/// Memory access methods available for RISC-V debug target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAccessMethod {
    Abstract,
    AbstAuto,
    Progbuf,
    Sysbus,
}

/// RISC-V Hart state and controller.
pub struct RiscvHart {
    pub dmi: CDmiTransport,
    pub isa: HartIsa,
    pub hart_idx: u32,
    pub hartsel: u32,
    pub vendor_id: u32,
    pub arch_id: u32,
    pub impl_id: u32,
    pub hart_id: u32,
    pub progbuf_size: u8,
    pub has_sysbus: bool,
    pub has_abstauto: bool,
    pub active_method: MemoryAccessMethod,
}

impl RiscvHart {
    /// Create a new Hart descriptor and discover core capabilities.
    pub fn probe(mut dmi: CDmiTransport, hart_idx: u32) -> Result<Self, DmiError> {
        let hartsel = hartsel_mask(hart_idx);

        // Select the hart and ensure active
        dmi.write(RV_DM_CONTROL, hartsel | RV_DM_CTRL_ACTIVE)?;
        let status = dmi.read(RV_DM_STATUS)?;
        if (status & (RV_DM_STAT_ANY_NONEXIST | RV_DM_STAT_ANY_UNAVAIL)) != 0 {
            return Err(DmiError::NotSupported);
        }

        // Discover progbuf size and data count
        let abstcs = dmi.read(RV_DM_ABSTRACTCS)?;
        let progbuf_size = ((abstcs & RV_DM_ABSTCS_PROGBUF_SIZE_MASK) >> RV_DM_ABSTCS_PROGBUF_SIZE_SHIFT) as u8;

        // Check sysbus support
        let sbcs = dmi.read(RV_DM_SBCS).unwrap_or(0);
        let has_sysbus = (sbcs & 0xfe0) != 0; // sbasize > 0 (bits 11:5)
        if has_sysbus {
            let _ = dmi.write(RV_DM_SBCS, 0x00407000); // clear sbbusyerror and sberror
        }

        // Check abstractauto support
        let has_abstauto = if dmi.write(RV_DM_ABSTRACTAUTO, 1).is_ok() {
            let auto = dmi.read(RV_DM_ABSTRACTAUTO).unwrap_or(0);
            let _ = dmi.write(RV_DM_ABSTRACTAUTO, 0);
            (auto & 1) != 0
        } else {
            false
        };

        let mut hart = Self {
            dmi,
            isa: HartIsa::Rv32i, // Temporary default
            hart_idx,
            hartsel,
            vendor_id: 0,
            arch_id: 0,
            impl_id: 0,
            hart_id: 0,
            progbuf_size,
            has_sysbus,
            has_abstauto,
            active_method: MemoryAccessMethod::Abstract,
        };

        // Halt hart briefly to read IDs and MISA safely
        hart.halt_request()?;

        // Discover ISA from MISA
        let misa = hart.csr_read(RV_CSR_MISA).unwrap_or(0);
        if (misa & RV_MISA_EXT_E) != 0 {
            hart.isa = HartIsa::Rv32e;
        } else if (misa & RV_MISA_EXT_F) != 0 {
            hart.isa = HartIsa::Rv32iSingleFpu;
        } else {
            hart.isa = HartIsa::Rv32i;
        }

        // Read identifiers
        hart.vendor_id = hart.csr_read(RV_CSR_MVENDORID).unwrap_or(0);
        hart.arch_id = hart.csr_read(RV_CSR_MARCHID).unwrap_or(0);
        hart.impl_id = hart.csr_read(RV_CSR_MIMPID).unwrap_or(0);
        hart.hart_id = hart.csr_read(RV_CSR_MHARTID).unwrap_or(0);

        Ok(hart)
    }

    /// Access the underlying DMI transport.
    pub fn dmi(&self) -> &CDmiTransport {
        &self.dmi
    }

    /// Access the underlying DMI transport mutably.
    pub fn dmi_mut(&mut self) -> &mut CDmiTransport {
        &mut self.dmi
    }

    /// Read a 32-bit CSR using abstract register access command.
    pub fn csr_read(&mut self, csr: u16) -> Result<u32, DmiError> {
        let cmd = make_reg_access_cmd(csr, false, true, false);
        exec_command(&mut self.dmi, cmd)?;
        self.dmi.read(RV_DM_DATA0)
    }

    /// Write a 32-bit CSR using abstract register access command.
    pub fn csr_write(&mut self, csr: u16, val: u32) -> Result<(), DmiError> {
        self.dmi.write(RV_DM_DATA0, val)?;
        let cmd = make_reg_access_cmd(csr, true, true, false);
        exec_command(&mut self.dmi, cmd)
    }

    /// Read a single GPR (0..31).
    pub fn gpr_read(&mut self, gpr: u8) -> Result<u32, DmiError> {
        self.csr_read(RV_REG_GPR_BASE + gpr as u16)
    }

    /// Write a single GPR (0..31).
    pub fn gpr_write(&mut self, gpr: u8, val: u32) -> Result<(), DmiError> {
        self.csr_write(RV_REG_GPR_BASE + gpr as u16, val)
    }

    /// Read program counter (DPC).
    pub fn pc_read(&mut self) -> Result<u32, DmiError> {
        self.csr_read(RV_CSR_DPC)
    }

    /// Write program counter (DPC).
    pub fn pc_write(&mut self, val: u32) -> Result<(), DmiError> {
        self.csr_write(RV_CSR_DPC, val)
    }

    /// Read a single register by GDB register number into `buf`.
    pub fn reg_read(&mut self, reg: u32, buf: &mut [u8]) -> Result<usize, TargetError> {
        if buf.len() < 4 {
            return Err(TargetError::InternalError);
        }
        let gprs_count = self.isa.gpr_count() as u32;

        let val = if reg < gprs_count {
            self.gpr_read(reg as u8).map_err(|_| TargetError::AccessFault)?
        } else if reg == gprs_count {
            self.pc_read().map_err(|_| TargetError::AccessFault)?
        } else if self.isa.has_single_fpu() && (RV_FPU_GDB_OFFSET..=64).contains(&reg) {
            // f0..f31 single-precision FPRs (0x1020..0x103f)
            let fpr = (reg - RV_FPU_GDB_OFFSET) as u16;
            self.csr_read(RV_REG_FPR_BASE + fpr).map_err(|_| TargetError::AccessFault)?
        } else if self.isa.has_single_fpu() && (RV_FPU_GDB_CSR_OFFSET..=68).contains(&reg) {
            // fflags (0x001), frm (0x002), fcsr (0x003)
            let csr = (reg - RV_FPU_GDB_CSR_OFFSET + 1) as u16;
            self.csr_read(csr).map_err(|_| TargetError::AccessFault)?
        } else if reg >= RV_CSR_GDB_OFFSET {
            let csr = (reg - RV_CSR_GDB_OFFSET) as u16;
            self.csr_read(csr).map_err(|_| TargetError::AccessFault)?
        } else {
            return Err(TargetError::Unsupported);
        };

        buf[..4].copy_from_slice(&val.to_le_bytes());
        Ok(4)
    }

    /// Write a single register by GDB register number from `data`.
    pub fn reg_write(&mut self, reg: u32, data: &[u8]) -> Result<usize, TargetError> {
        if data.len() < 4 {
            return Err(TargetError::InternalError);
        }
        let val = u32::from_le_bytes(data[..4].try_into().unwrap());
        let gprs_count = self.isa.gpr_count() as u32;

        if reg < gprs_count {
            self.gpr_write(reg as u8, val).map_err(|_| TargetError::AccessFault)?;
        } else if reg == gprs_count {
            self.pc_write(val).map_err(|_| TargetError::AccessFault)?;
        } else if self.isa.has_single_fpu() && (RV_FPU_GDB_OFFSET..=64).contains(&reg) {
            let fpr = (reg - RV_FPU_GDB_OFFSET) as u16;
            self.csr_write(RV_REG_FPR_BASE + fpr, val).map_err(|_| TargetError::AccessFault)?;
        } else if self.isa.has_single_fpu() && (RV_FPU_GDB_CSR_OFFSET..=68).contains(&reg) {
            let csr = (reg - RV_FPU_GDB_CSR_OFFSET + 1) as u16;
            self.csr_write(csr, val).map_err(|_| TargetError::AccessFault)?;
        } else if reg >= RV_CSR_GDB_OFFSET {
            let csr = (reg - RV_CSR_GDB_OFFSET) as u16;
            self.csr_write(csr, val).map_err(|_| TargetError::AccessFault)?;
        } else {
            return Err(TargetError::Unsupported);
        }

        Ok(4)
    }

    /// Read all general registers (GPRs + PC) into `buf`.
    pub fn regs_read(&mut self, buf: &mut [u8]) -> Result<(), TargetError> {
        let count = self.isa.gpr_count();
        let total_size = (count + 1) * 4;
        if buf.len() < total_size {
            return Err(TargetError::InternalError);
        }

        for i in 0..count {
            let val = self.gpr_read(i as u8).map_err(|_| TargetError::AccessFault)?;
            buf[i * 4..(i + 1) * 4].copy_from_slice(&val.to_le_bytes());
        }
        let pc = self.pc_read().map_err(|_| TargetError::AccessFault)?;
        buf[count * 4..(count + 1) * 4].copy_from_slice(&pc.to_le_bytes());

        Ok(())
    }

    /// Write all general registers (GPRs + PC) from `data`.
    pub fn regs_write(&mut self, data: &[u8]) -> Result<(), TargetError> {
        let count = self.isa.gpr_count();
        let total_size = (count + 1) * 4;
        if data.len() < total_size {
            return Err(TargetError::InternalError);
        }

        for i in 0..count {
            let val = u32::from_le_bytes(data[i * 4..(i + 1) * 4].try_into().unwrap());
            self.gpr_write(i as u8, val).map_err(|_| TargetError::AccessFault)?;
        }
        let pc = u32::from_le_bytes(data[count * 4..(count + 1) * 4].try_into().unwrap());
        self.pc_write(pc).map_err(|_| TargetError::AccessFault)?;

        Ok(())
    }

    /// Request halt of target core.
    pub fn halt_request(&mut self) -> Result<(), DmiError> {
        self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE | RV_DM_CTRL_HALT_REQ)?;
        for _ in 0..1000 {
            let status = self.dmi.read(RV_DM_STATUS)?;
            if (status & RV_DM_STAT_ALL_HALTED) != 0 {
                // Clear haltreq once halted, matching C Blackmagic line 1337 and sdi.txt Table 3-16
                self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE)?;
                return Ok(());
            }
        }
        let _ = self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE);
        Err(DmiError::Timeout)
    }

    /// Resume core execution, optionally single-stepping.
    pub fn halt_resume(&mut self, step: bool) -> Result<(), DmiError> {
        // Read dcsr and adjust step bit
        let mut dcsr = self.csr_read(RV_CSR_DCSR).unwrap_or(0);
        if step {
            dcsr |= RV_DCSR_STEP;
        } else {
            dcsr &= !RV_DCSR_STEP;
        }
        // Always ensure ebreakm is set (patch 104)
        dcsr |= RV_DCSR_EBREAKM;
        let _ = self.csr_write(RV_CSR_DCSR, dcsr);

        // Step 1: Clear haltreq before resume (sdi.txt Table 3-16)
        self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE)?;

        // Step 2: Send resume request (0x40000001)
        self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE | RV_DM_CTRL_HART_RESUME_REQ)?;
        for _ in 0..1000 {
            let status = self.dmi.read(RV_DM_STATUS)?;
            if (status & RV_DM_STAT_ALL_RUNNING) != 0 || (status & RV_DM_STAT_ALL_HALTED) == 0 {
                // Target is running
                let _ = self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE);
                return Ok(());
            }
            // Check if core already ran and halted on ebreak/step
            let dcsr = self.csr_read(RV_CSR_DCSR).unwrap_or(0);
            let cause = (dcsr & RV_DCSR_CAUSE_MASK) >> RV_DCSR_CAUSE_SHIFT;
            if cause == 1 /* ebreak */ || (step && cause == 4 /* step */) {
                let _ = self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE);
                return Ok(());
            }
        }
        let _ = self.dmi.write(RV_DM_CONTROL, self.hartsel | RV_DM_CTRL_ACTIVE);
        Err(DmiError::Timeout)
    }

    /// Poll core execution state.
    pub fn halt_poll(&mut self) -> (HaltReason, u32) {
        let status = match self.dmi.read(RV_DM_STATUS) {
            Ok(s) => s,
            Err(_) => return (HaltReason::Error, 0),
        };

        if (status & RV_DM_STAT_ALL_HALTED) != 0 {
            let dcsr = self.csr_read(RV_CSR_DCSR).unwrap_or(0);
            let cause = (dcsr & RV_DCSR_CAUSE_MASK) >> RV_DCSR_CAUSE_SHIFT;
            let reason = match cause {
                1 => HaltReason::Breakpoint, // ebreak
                2 => HaltReason::Watchpoint, // trigger
                3 => HaltReason::Request,    // haltreq
                4 => HaltReason::Stepping,   // step
                _ => HaltReason::Unknown,
            };
            (reason, 0)
        } else if (status & RV_DM_STAT_ALL_RUNNING) != 0 {
            (HaltReason::Running, 0)
        } else {
            (HaltReason::Unknown, 0)
        }
    }

    /// Reset target core with halt request asserted (patch 101).
    pub fn reset(&mut self) -> Result<(), DmiError> {
        // Assert haltreq and ndmreset together
        self.dmi.write(
            RV_DM_CONTROL,
            self.hartsel | RV_DM_CTRL_ACTIVE | RV_DM_CTRL_HALT_REQ | RV_DM_CTRL_NDM_RESET,
        )?;

        // Release ndmreset while maintaining haltreq
        self.dmi.write(
            RV_DM_CONTROL,
            self.hartsel | RV_DM_CTRL_ACTIVE | RV_DM_CTRL_HALT_REQ,
        )?;

        // Acknowledge the reset while holding haltreq
        self.dmi.write(
            RV_DM_CONTROL,
            self.hartsel | RV_DM_CTRL_ACTIVE | RV_DM_CTRL_HALT_REQ | RV_DM_CTRL_HART_ACK_RESET,
        )?;

        // Ensure halted
        self.halt_request()?;

        // Set ebreakm in dcsr so software breakpoints trigger debug halt
        let mut dcsr = self.csr_read(RV_CSR_DCSR).unwrap_or(0);
        dcsr |= RV_DCSR_EBREAKM;
        let _ = self.csr_write(RV_CSR_DCSR, dcsr);

        Ok(())
    }

    // --- Direct DM / DMI access ---
    #[inline]
    pub fn dm_read(&mut self, addr: u8) -> Result<u32, DmiError> {
        self.dmi.read(addr)
    }

    #[inline]
    pub fn dm_write(&mut self, addr: u8, val: u32) -> Result<(), DmiError> {
        self.dmi.write(addr, val)
    }

    // --- Direct Memory Access (Native Rust API) ---
    pub fn mem_read(&mut self, addr: u32, dest: &mut [u8]) -> Result<(), TargetError> {
        super::mem::RiscvMem::read(self, addr, dest).map_err(|_| TargetError::AccessFault)
    }

    pub fn mem_write(&mut self, addr: u32, src: &[u8]) -> Result<(), TargetError> {
        super::mem::RiscvMem::write(self, addr, src).map_err(|_| TargetError::AccessFault)
    }

    pub fn mem_read_nostop(&mut self, addr: u32, dest: &mut [u8]) -> Result<bool, TargetError> {
        super::mem::RiscvMem::read_nostop(self, addr, dest).map_err(|_| TargetError::AccessFault)
    }

    pub fn mem_write_nostop(&mut self, addr: u32, src: &[u8]) -> Result<bool, TargetError> {
        super::mem::RiscvMem::write_nostop(self, addr, src).map_err(|_| TargetError::AccessFault)
    }

    pub fn mem_read32(&mut self, addr: u32) -> Result<u32, TargetError> {
        let mut buf = [0u8; 4];
        self.mem_read(addr, &mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    pub fn mem_read16(&mut self, addr: u32) -> Result<u16, TargetError> {
        let mut buf = [0u8; 2];
        self.mem_read(addr, &mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    pub fn mem_write32(&mut self, addr: u32, val: u32) -> Result<(), TargetError> {
        self.mem_write(addr, &val.to_le_bytes())
    }

    pub fn mem_write16(&mut self, addr: u32, val: u16) -> Result<(), TargetError> {
        self.mem_write(addr, &val.to_le_bytes())
    }

    // --- Stub Execution (Native Rust API) ---
    pub fn run_stub(&mut self, loadaddr: u32, p1: u32, p2: u32, p3: u32, p4: u32) -> bool {
        super::stub::run_stub(self, loadaddr, p1, p2, p3, p4)
    }

    pub fn run_stub_with_stack(&mut self, loadaddr: u32, stack: u32, p1: u32, p2: u32, p3: u32, p4: u32) -> bool {
        super::stub::run_stub_with_stack(self, loadaddr, stack, p1, p2, p3, p4)
    }
}
