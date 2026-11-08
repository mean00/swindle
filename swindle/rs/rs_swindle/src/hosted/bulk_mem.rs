//! Bulk target memory access over the RPC link (hosted only).
//!
//! A RISC-V DM memory access costs ~11 DMI transactions per word, and a DMI
//! transaction used to be one RPC round trip of its own: 16 KiB of `load`
//! payload cost ~45k round trips, i.e. ~2 KB/s however fast the wire is. The
//! `RV.MEM_READ` / `RV.MEM_WRITE` leaves carry a whole block per frame and let
//! the probe run the same access sequence, so the wire cost is the payload
//! (~176 µs per 256-byte block instead of ~176 µs per DMI op).
//!
//! The access method is the one the scan selected (`hart.active_method`), so a
//! block is served by the same abstract / autoinc / sysbus sequence the host
//! would otherwise have run op by op.

use crate::SyncCell;
use crate::hosted::dmi_scan::current_leg;
use crate::hosted::rpc_host_generated::{remote_rv_mem_read, remote_rv_mem_write};
use crate::hosted::transport::RPC_BUFFER_SIZE;
use crate::riscv::hart::MemoryAccessMethod;
use crate::riscv::mem::MEM_BLOCK_MAX;
use crate::target::driver::TargetError;

/// Bytes per frame: the protocol's block size ([`MEM_BLOCK_MAX`]).
///
/// The probe checks the same constant against its own buffers; here the reply buffer
/// has to hold a full block with every byte escaped (twice its size) plus the framing.
const BLOCK_BYTES: usize = MEM_BLOCK_MAX;
const _: () = assert!(2 * BLOCK_BYTES + 64 <= RPC_BUFFER_SIZE);

fn leg() -> u32 {
    u32::from(current_leg().wire_id())
}

/// Access memory one DM operation at a time (`swindle_hosted --raw-dmi`).
///
/// The block leaves are the default, and what make a hosted `load` usable. With this
/// switched on, `RiscvHart::mem_read` / `mem_write` run the access sequence on the host
/// instead, i.e. one RPC round trip per DMI op - slow, but it keeps working
/// against a probe that does not know `RV.MEM_*`, and it makes the two ways of
/// driving the same sequence comparable on the same target.
static RAW_DMI: SyncCell<bool> = SyncCell::new(false);

/// Set the access mode (called by the CLI flag before the server starts).
pub fn set_raw_dmi(on: bool) {
    RAW_DMI.set(on);
}

/// Whether memory accesses go over the per-op DMI leaves.
pub fn raw_dmi() -> bool {
    RAW_DMI.get()
}

/// `--raw-dmi` / `swindle_hosted`'s C side (`hosted/src/main.cpp`).
#[unsafe(no_mangle)]
pub extern "C" fn swindleSetRawDmi_c(on: i32) {
    set_raw_dmi(on != 0);
}

/// Write `src` to `addr`, one frame per block.
pub fn write(method: MemoryAccessMethod, addr: u32, src: &[u8]) -> Result<(), TargetError> {
    let mut offset = 0;
    while offset < src.len() {
        let end = (offset + BLOCK_BYTES).min(src.len());
        let (fault, _) = remote_rv_mem_write(
            method.wire_id() as u32,
            leg(),
            addr + offset as u32,
            (end - offset) as u32,
            &src[offset..end],
        );
        if fault != 0 {
            return Err(TargetError::AccessFault);
        }
        offset = end;
    }
    Ok(())
}

/// Read `dest.len()` bytes from `addr`, one frame per block.
pub fn read(method: MemoryAccessMethod, addr: u32, dest: &mut [u8]) -> Result<(), TargetError> {
    let mut offset = 0;
    while offset < dest.len() {
        let end = (offset + BLOCK_BYTES).min(dest.len());
        let want = end - offset;
        let (fault, data) =
            remote_rv_mem_read(method.wire_id() as u32, leg(), addr + offset as u32, want as u32);
        // A short block is a transport problem, not a target one: report it
        // rather than leaving part of `dest` stale.
        if fault != 0 || data.len() != want {
            return Err(TargetError::AccessFault);
        }
        dest[offset..end].copy_from_slice(data);
        offset = end;
    }
    Ok(())
}
