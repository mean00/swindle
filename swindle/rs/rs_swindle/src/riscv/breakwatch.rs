//! RISC-V hardware breakpoints and watchpoints, programmed through the trigger
//! unit (`tselect` / `tdata1` / `tdata2`).
//!
//! Port of `riscv_config_trigger32()`, `riscv32_breakwatch_set()` and
//! `riscv32_breakwatch_clear()` from `blackmagic/src/target/riscv32.c`. The
//! mechanism is generic to every RISC-V hart that implements an address-match
//! trigger; *whether* it has one is the target-specific part, and that answer
//! already lives in [`super::triggers::HwTriggers`] / [`RiscvHart::triggers`]:
//! a hart with no usable slot reports no hardware breakpoints, and this module
//! then fails the request (GDB falls back to patching the code).
//!
//! An execute breakpoint becomes `mcontrol` (tdata1 type 2) with the execute bit
//! set, a watchpoint the same trigger with the load/store bits set, and the
//! address goes into `tdata2`. A trigger that fires enters debug mode (`action` =
//! 0), which is what GDB sees as a halt.
//!
//! Harts differ in what they accept: a CH32V303 keeps only the execute bit and
//! drops `load`/`store`, so a watchpoint there fails with
//! [`TargetError::Unsupported`] (GDB reports it instead of corrupting memory),
//! while its execute breakpoints work over the trigger unit. A CH32V003/006 has
//! no trigger unit at all (`tselect`/`tdata1` read back 0 and ignore writes), so
//! that hart never advertises a usable slot and its `hbreak` is a code patch.
//!
//! Nothing is remembered on our side: the trigger CSRs *are* the table. A slot is
//! free when its `tdata1` matches nothing, "already armed" is a `tdata1`/`tdata2`
//! read-back, and a request that finds neither fails with
//! [`TargetError::NoResources`] so the GDB handler patches the code instead. That
//! keeps this out of the `.bss` of builds that have no room left for a 16-entry
//! table, and it stays correct when the target programs a trigger of its own.

use super::hart::RiscvHart;
use super::rv_csr::{RV_CSR_TDATA1, RV_CSR_TDATA2, RV_CSR_TSELECT};
use super::triggers::{RV_TRIGGER_SUPPORT_MCONTROL, RV_TRIGGER_SUPPORT_MCONTROL6};
use crate::target::{Breakwatch, BreakwatchType, TargetError};

// --- mcontrol (tdata1, trigger type 2) fields, RV32 layout ---
// See §5.2.9 of the RISC-V debug spec v0.13.2; the values are the ones the C
// port uses (RV32_MATCH_* in riscv32.c) so both paths program identical harts.

/// Trigger type 2 (`mcontrol`) in `tdata1` bits 31:28.
const RV32_MATCH_TYPE_MCONTROL: u32 = 0x2000_0000;
/// `dmode`: the trigger also acts while the hart is halted.
const RV32_MATCH_DMODE_DEBUG: u32 = 0x0800_0000;
/// `execute`: match an instruction fetch.
const RV32_MATCH_EXECUTE: u32 = 0x0000_0004;
/// `load`: match a read.
const RV32_MATCH_READ: u32 = 0x0000_0001;
/// `store`: match a write.
const RV32_MATCH_WRITE: u32 = 0x0000_0002;
/// Match in every privilege mode (M/S/U).
const RV32_MATCH_PRIV_ALL: u32 = 0x0000_0058;
/// `action` = 1: enter debug mode instead of raising an exception (Debug Spec 0.13.2 §5.2.9).
const RV32_MATCH_ACTION_DEBUG: u32 = 0x0000_1000;
/// `match` = 0: compare `tdata2` against the access address.
const RV32_MATCH_SELECT_ADDR: u32 = 0x0000_0000;
/// `timing` = 0: match before the instruction executes.
const RV32_MATCH_BEFORE: u32 = 0x0000_0000;
/// `timing` = 1: match after the instruction executes.
const RV32_MATCH_AFTER: u32 = 0x0004_0000;
/// The R/W/X bits of `mcontrol`: the part a write must read back identically.
const RV32_MATCH_ACCESS_BITS: u32 = 0x7;

// --- `size` field of `mcontrol` (bits 22:21 + 17:16, the upper two are RV64) ---
// Mirrors `riscv_match_size_e`. Zero means "match any size at this address",
// which is what an execute trigger needs and what the C port ends up writing for
// the sizes GDB sends (see [`match_size`]).

const RV_MATCH_SIZE_8_BIT: u32 = 0x0001_0000;
const RV_MATCH_SIZE_16_BIT: u32 = 0x0002_0000;
const RV_MATCH_SIZE_32_BIT: u32 = 0x0003_0000;
const RV_MATCH_SIZE_48_BIT: u32 = 0x0020_0000;
const RV_MATCH_SIZE_64_BIT: u32 = 0x0021_0000;
const RV_MATCH_SIZE_80_BIT: u32 = 0x0022_0000;
const RV_MATCH_SIZE_96_BIT: u32 = 0x0023_0000;
const RV_MATCH_SIZE_112_BIT: u32 = 0x0040_0000;
const RV_MATCH_SIZE_128_BIT: u32 = 0x0041_0000;

/// `size` field of `mcontrol` (bits 22:21 + 17:16).
const RV32_MATCH_SIZE_MASK: u32 = 0x0063_0000;

/// The R/W/X and `size` bits of `tdata1`: together with `tdata2` they say *what*
/// a slot matches, which is how an armed trigger is recognised again.
const RV32_MATCH_ID_MASK: u32 = RV32_MATCH_ACCESS_BITS | RV32_MATCH_SIZE_MASK;

/// `mcontrol` `size` field for a watched width.
///
/// The C table (`riscv_breakwatch_match_size()`) is keyed on a *bit* count, but the
/// GDB breakpoint path hands it the byte count, so the usual 4-byte request lands on
/// the "no size bits" arm. That is what C targets run with, and it is also what an
/// execute trigger needs (its size field must be zero), so it is kept as is.
fn match_size(size: usize) -> u32 {
    match size {
        8 => RV_MATCH_SIZE_8_BIT,
        16 => RV_MATCH_SIZE_16_BIT,
        32 => RV_MATCH_SIZE_32_BIT,
        48 => RV_MATCH_SIZE_48_BIT,
        64 => RV_MATCH_SIZE_64_BIT,
        80 => RV_MATCH_SIZE_80_BIT,
        96 => RV_MATCH_SIZE_96_BIT,
        112 => RV_MATCH_SIZE_112_BIT,
        128 => RV_MATCH_SIZE_128_BIT,
        _ => 0,
    }
}

/// The `mcontrol` configuration word for a breakwatch, or `None` when the type
/// cannot be a hardware trigger at all (a software breakpoint is a code patch).
fn config_for(kind: BreakwatchType, size: usize) -> Option<u32> {
    let common = RV32_MATCH_TYPE_MCONTROL
        | RV32_MATCH_DMODE_DEBUG
        | RV32_MATCH_PRIV_ALL
        | RV32_MATCH_ACTION_DEBUG
        | RV32_MATCH_SELECT_ADDR
        | match_size(size);
    match kind {
        // Break on the fetch of the address itself.
        BreakwatchType::BreakHard => Some(common | RV32_MATCH_EXECUTE | RV32_MATCH_BEFORE),
        // A read watchpoint has to report the value the load saw, hence "after".
        BreakwatchType::WatchRead => Some(common | RV32_MATCH_READ | RV32_MATCH_AFTER),
        BreakwatchType::WatchWrite => Some(common | RV32_MATCH_WRITE | RV32_MATCH_BEFORE),
        BreakwatchType::WatchAccess => {
            Some(common | RV32_MATCH_READ | RV32_MATCH_WRITE | RV32_MATCH_AFTER)
        }
        BreakwatchType::BreakSoft => None,
    }
}

/// True if `slot` advertises an address-match trigger (`mcontrol`/`mcontrol6`).
fn slot_can_match(hart: &RiscvHart, slot: u8) -> bool {
    u32::from(hart.triggers.uses[slot as usize])
        & (RV_TRIGGER_SUPPORT_MCONTROL | RV_TRIGGER_SUPPORT_MCONTROL6)
        != 0
}

/// The `(tdata1, tdata2)` pair a trigger slot currently holds.
fn read_slot(hart: &mut RiscvHart, slot: u8) -> Result<(u32, u32), TargetError> {
    hart.csr_write(RV_CSR_TSELECT, u32::from(slot))
        .map_err(|_| TargetError::AccessFault)?;
    let tdata1 = hart
        .csr_read(RV_CSR_TDATA1)
        .map_err(|_| TargetError::AccessFault)?;
    let tdata2 = hart
        .csr_read(RV_CSR_TDATA2)
        .map_err(|_| TargetError::AccessFault)?;
    Ok((tdata1, tdata2))
}

/// True if the trigger holds nothing to match (`tdata1` with no access bits).
fn slot_is_idle(tdata1: u32) -> bool {
    tdata1 & RV32_MATCH_ACCESS_BITS == 0
}

/// Program one trigger slot, verifying that the hart accepted the match mode.
fn config_trigger(
    hart: &mut RiscvHart,
    slot: u8,
    config: u32,
    address: u32,
) -> Result<(), TargetError> {
    hart.csr_write(RV_CSR_TSELECT, u32::from(slot))
        .map_err(|_| TargetError::AccessFault)?;
    hart.csr_write(RV_CSR_TDATA1, config)
        .map_err(|_| TargetError::AccessFault)?;
    hart.csr_write(RV_CSR_TDATA2, address)
        .map_err(|_| TargetError::AccessFault)?;

    let read_back = hart
        .csr_read(RV_CSR_TDATA1)
        .map_err(|_| TargetError::AccessFault)?;
    if read_back & RV32_MATCH_ACCESS_BITS == config & RV32_MATCH_ACCESS_BITS {
        return Ok(());
    }
    // A hart that refused the configuration must not be left half-programmed.
    let _ = hart.csr_write(RV_CSR_TDATA1, 0);
    Err(TargetError::Unsupported)
}

/// Turn a slot back into an unconfigured `mcontrol` trigger (tdata1 = 0).
fn disarm_slot(hart: &mut RiscvHart, slot: u8) -> Result<(), TargetError> {
    hart.csr_write(RV_CSR_TSELECT, u32::from(slot))
        .map_err(|_| TargetError::AccessFault)?;
    hart.csr_write(RV_CSR_TDATA1, 0)
        .map_err(|_| TargetError::AccessFault)
}

/// Arm `breakwatch` on the trigger unit of `hart`.
///
/// Fails with [`TargetError::Unsupported`] when the type cannot be a trigger, with
/// [`TargetError::NoResources`] when the hart has no trigger that can match an
/// address or all of them are taken, and with [`TargetError::AccessFault`] when the
/// CSRs cannot be reached -- all of which make the GDB handler fall back to
/// patching the code.
pub fn set(hart: &mut RiscvHart, breakwatch: &mut Breakwatch) -> Result<(), TargetError> {
    let (kind, addr, size) = (breakwatch.kind(), breakwatch.addr(), breakwatch.size());
    let config = config_for(kind, size).ok_or(TargetError::Unsupported)?;

    let mut free = None;
    for slot in 0..hart.triggers.slots {
        if !slot_can_match(hart, slot) {
            continue;
        }
        let (tdata1, tdata2) = read_slot(hart, slot)?;
        if tdata1 & RV32_MATCH_ID_MASK == config & RV32_MATCH_ID_MASK && tdata2 == addr {
            // Already armed: GDB re-sends every `Z` packet on each resume.
            breakwatch.set_slot(slot);
            return Ok(());
        }
        if free.is_none() && slot_is_idle(tdata1) {
            free = Some(slot);
        }
    }

    let slot = free.ok_or(TargetError::NoResources)?;
    config_trigger(hart, slot, config, addr)?;
    breakwatch.set_slot(slot);
    Ok(())
}

/// Disarm the trigger `breakwatch` names.
pub fn clear(hart: &mut RiscvHart, breakwatch: &Breakwatch) -> Result<(), TargetError> {
    let (kind, addr, size) = (breakwatch.kind(), breakwatch.addr(), breakwatch.size());
    let config = config_for(kind, size).ok_or(TargetError::Unsupported)?;

    // Fast path: if the breakwatch recorded its slot from set(), check that slot directly
    if let Some(slot) = breakwatch.slot() {
        if slot < hart.triggers.slots {
            let (tdata1, tdata2) = read_slot(hart, slot)?;
            if tdata1 & RV32_MATCH_ID_MASK == config & RV32_MATCH_ID_MASK && tdata2 == addr {
                return disarm_slot(hart, slot);
            }
        }
    }

    for slot in 0..hart.triggers.slots {
        let (tdata1, tdata2) = read_slot(hart, slot)?;
        if tdata1 & RV32_MATCH_ID_MASK == config & RV32_MATCH_ID_MASK && tdata2 == addr {
            return disarm_slot(hart, slot);
        }
    }
    Err(TargetError::NotFound)
}

/// Zero every trigger slot the hart advertises (attach and detach paths).
///
/// C `riscv_attach()` does the same: a trigger left armed by an earlier session
/// would stop the hart for a reason GDB knows nothing about.
pub fn clear_all(hart: &mut RiscvHart) {
    let slots = hart.triggers.slots;
    for slot in 0..slots {
        let _ = disarm_slot(hart, slot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_KINDS: [BreakwatchType; 4] = [
        BreakwatchType::BreakHard,
        BreakwatchType::WatchRead,
        BreakwatchType::WatchWrite,
        BreakwatchType::WatchAccess,
    ];

    #[test]
    fn execute_trigger_matches_the_fetch() {
        let config = config_for(BreakwatchType::BreakHard, 4).unwrap();
        assert_eq!(config & RV32_MATCH_ACCESS_BITS, RV32_MATCH_EXECUTE);
        assert_ne!(config & RV32_MATCH_DMODE_DEBUG, 0);
    }

    #[test]
    fn watchpoints_pick_their_access_bits() {
        let read = config_for(BreakwatchType::WatchRead, 4).unwrap();
        assert_eq!(read & RV32_MATCH_ACCESS_BITS, RV32_MATCH_READ);
        assert_ne!(read & RV32_MATCH_AFTER, 0);

        let write = config_for(BreakwatchType::WatchWrite, 4).unwrap();
        assert_eq!(write & RV32_MATCH_ACCESS_BITS, RV32_MATCH_WRITE);
        assert_eq!(write & RV32_MATCH_AFTER, RV32_MATCH_BEFORE);

        let access = config_for(BreakwatchType::WatchAccess, 4).unwrap();
        assert_eq!(
            access & RV32_MATCH_ACCESS_BITS,
            RV32_MATCH_READ | RV32_MATCH_WRITE
        );
    }

    #[test]
    fn software_breakpoints_are_not_a_trigger() {
        assert_eq!(config_for(BreakwatchType::BreakSoft, 4), None);
    }

    #[test]
    fn every_trigger_enters_debug_mode() {
        for kind in ALL_KINDS {
            let config = config_for(kind, 4).unwrap();
            assert_eq!(config & 0xf000_0000, RV32_MATCH_TYPE_MCONTROL);
            assert_ne!(config & RV32_MATCH_ACTION_DEBUG, 0);
        }
    }

    /// The width GDB asks for is a byte length, which the size table (keyed on
    /// bits, as in C) does not know: the trigger is programmed as an address match
    /// of any size instead.
    #[test]
    fn gdb_word_length_has_no_size_bits() {
        assert_eq!(match_size(4), 0);
        assert_eq!(match_size(8), RV_MATCH_SIZE_8_BIT);
        assert_eq!(match_size(64), RV_MATCH_SIZE_64_BIT);
    }
}
