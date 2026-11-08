// rpc_rv_impl.rs — RV class command implementations
// Called by rpc_target_generated.rs

use crate::bmp;
use crate::native::rpc_target::rpc_reply::REPLY_TEMP_BUFFER_SIZE;
use crate::packet_symbols::INPUT_BUFFER_SIZE;
use crate::riscv::dmi::{DmiError, DmiLeg, DmiTransport};
use crate::riscv::hart::MemoryAccessMethod;
use crate::riscv::mem::{MEM_BLOCK_MAX, RiscvMem};

/// Fault code for a request this build cannot serve (unknown access method or
/// leg, a block bigger than [`MEM_BLOCK_MAX`] or one whose data does not match its length).
const FAULT_UNSUPPORTED: i32 = -1;
/// Fault code for a DMI transaction that failed on the wire.
const FAULT_ACCESS: i32 = 1;

// A full block has to fit the probe's buffers: the data buffer the reply is streamed
// from, and the request frame (escaped payload, at worst twice its size, plus header).
const _: () = assert!(MEM_BLOCK_MAX <= REPLY_TEMP_BUFFER_SIZE);
const _: () = assert!(2 * MEM_BLOCK_MAX + 64 <= INPUT_BUFFER_SIZE);

/// The probe's own DM transport: one DMI transaction per call, straight out on
/// the wire — the same entry points the `DM_READ` / `DM_WRITE` leaves use, for
/// the leg the host selected.
struct ProbeDmi {
    leg: DmiLeg,
}

impl DmiTransport for ProbeDmi {
    fn read(&mut self, addr: u8) -> Result<u32, DmiError> {
        let (ok, value) = match self.leg {
            DmiLeg::Rvswd => bmp::bmp_rv_read(addr),
            DmiLeg::Sdi => crate::riscv_extra::bmp_sdi_read(addr),
        };
        if ok { Ok(value) } else { Err(DmiError::Fault) }
    }

    fn write(&mut self, addr: u8, val: u32) -> Result<(), DmiError> {
        let ok = match self.leg {
            DmiLeg::Rvswd => bmp::bmp_rv_write(addr, val),
            DmiLeg::Sdi => crate::riscv_extra::bmp_sdi_write(addr, val),
        };
        if ok { Ok(()) } else { Err(DmiError::Fault) }
    }

    /// The SDI wire cannot report a lost access, so reads are confirmed exactly as the
    /// host does for its own SDI transport (`read_confirm` in `riscv_dmi_s`).
    fn needs_read_confirm(&self) -> bool {
        self.leg == DmiLeg::Sdi
    }
}

/// Reset RISC-V DM
pub fn reset() -> (bool, u32) {
    bmp::rv_dm_start();
    (true, 0)
}

/// Read RISC-V DM register
pub fn dm_read(address: u32) -> (bool, u32) {
    bmp::bmp_rv_read(address as u8)
}

/// Write RISC-V DM register
pub fn dm_write(address: u32, value: u32) -> (bool, u32) {
    let ok = bmp::bmp_rv_write(address as u8, value);
    (ok, 0)
}

/// Read a target memory block, returning `(fault, bytes_read)`.
pub fn mem_read(method: u32, leg: u32, address: u32, length: u32, buffer: &mut [u8]) -> (i32, usize) {
    let len = length as usize;
    if len > MEM_BLOCK_MAX || len > buffer.len() {
        return (FAULT_UNSUPPORTED, 0);
    }
    let Some(method) = MemoryAccessMethod::from_wire_id(method) else {
        return (FAULT_UNSUPPORTED, 0);
    };
    let Some(leg) = DmiLeg::from_wire_id(leg) else {
        return (FAULT_UNSUPPORTED, 0);
    };
    let mut dmi = ProbeDmi { leg };
    match RiscvMem::read_with(&mut dmi, method, address, &mut buffer[..len]) {
        Ok(()) => (0, len),
        Err(_) => (FAULT_ACCESS, 0),
    }
}

/// Write a target memory block, returning `(fault, 0)`.
pub fn mem_write(method: u32, leg: u32, address: u32, length: u32, data: &[u8]) -> (i32, u32) {
    let len = length as usize;
    if len > MEM_BLOCK_MAX || len != data.len() {
        return (FAULT_UNSUPPORTED, 0);
    }
    let Some(method) = MemoryAccessMethod::from_wire_id(method) else {
        return (FAULT_UNSUPPORTED, 0);
    };
    let Some(leg) = DmiLeg::from_wire_id(leg) else {
        return (FAULT_UNSUPPORTED, 0);
    };
    let mut dmi = ProbeDmi { leg };
    match RiscvMem::write_with(&mut dmi, method, address, data) {
        Ok(()) => (0, 0),
        Err(_) => (FAULT_ACCESS, 0),
    }
}
