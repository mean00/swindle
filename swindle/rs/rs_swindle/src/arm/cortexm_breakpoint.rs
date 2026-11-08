//! Cortex-M hardware breakpoints (FPB) and watchpoints (DWT).

use super::cortex::{CORTEXM_TOPT_FLAVOUR_V6M, CORTEXM_TOPT_FLAVOUR_V8M};
use super::cortexm::{priv_ref, with_priv};
use super::cortexm_memaccess::{read32, write32};
use super::cortexm_registers::*;
use crate::target::breakwatch::{Breakwatch, MAX_BREAKWATCHES};
use crate::target::{BreakwatchType, Target, TargetError, TargetView};

/// Address of the FPB comparator `index`.
pub(super) fn cortexm_fpb_comp(index: u32) -> u32 {
    CORTEXM_FPB_BASE + CORTEXM_FPB_COMP_OFFSET + CORTEXM_FPB_COMP_STRIDE * index
}

/// Address of the DWT comparator `index`.
pub(super) fn cortexm_dwt_comp(index: u32) -> u32 {
    CORTEXM_DWT_BASE + CORTEXM_DWT_COMP_OFFSET + CORTEXM_DWT_SLOT_STRIDE * index
}

/// Address of the DWT mask register of comparator `index`.
pub(super) fn cortexm_dwt_mask(index: u32) -> u32 {
    CORTEXM_DWT_BASE + CORTEXM_DWT_MASK_OFFSET + CORTEXM_DWT_SLOT_STRIDE * index
}

/// Address of the DWT function register of comparator `index`.
pub(super) fn cortexm_dwt_func(index: u32) -> u32 {
    CORTEXM_DWT_BASE + CORTEXM_DWT_FUNC_OFFSET + CORTEXM_DWT_SLOT_STRIDE * index
}

/// DWT mask for a watchpoint of `len` bytes: the number of low address bits to ignore
/// (C `cortexm_dwt_mask()`), capped at `CORTEXM_DWT_MASK_MAX`.
fn dwt_mask(len: usize) -> u32 {
    if len < 2 {
        return 0;
    }
    ((len - 1) as u32).ilog2().min(CORTEXM_DWT_MASK_MAX)
}

/// DWTv1 function value for a watchpoint type (C `cortexm_dwt_func()`); `u32::MAX` for a
/// type this unit cannot express.
fn dwt_func(target: *mut Target, watch_type: BreakwatchType) -> u32 {
    let mut value = 0;
    if TargetView::new(target).options() & CORTEXM_TOPT_FLAVOUR_V6M == 0 {
        value = CORTEXM_DWT_FUNC_DATAVSIZE_WORD;
    }
    match watch_type {
        BreakwatchType::WatchWrite => CORTEXM_DWT_FUNC_FUNC_WRITE | value,
        BreakwatchType::WatchRead => CORTEXM_DWT_FUNC_FUNC_READ | value,
        BreakwatchType::WatchAccess => CORTEXM_DWT_FUNC_FUNC_ACCESS | value,
        _ => u32::MAX,
    }
}

/// DWTv2 function value for a watchpoint type and length (C `cortexm_dwtv2_func()`).
fn dwtv2_func(watch_type: BreakwatchType, len: usize) -> u32 {
    let value = CORTEXM_DWTV2_FUNC_ACTION_DEBUG_EVENT
        | (((len >> 1) as u32) << CORTEXM_DWTV2_FUNC_LEN_SHIFT);
    match watch_type {
        BreakwatchType::WatchWrite => value | CORTEXM_DWTV2_FUNC_MATCH_WRITE,
        BreakwatchType::WatchRead => value | CORTEXM_DWTV2_FUNC_MATCH_READ,
        BreakwatchType::WatchAccess => value | CORTEXM_DWTV2_FUNC_MATCH_ACCESS,
        _ => 0,
    }
}

/// Take the first free comparator slot out of `available` and mark it in `mask`
/// (the slot hunt in C `cortexm_breakpoint_set()` / `cortexm_watchpoint_set()`).
///
/// `None` means every slot is in use, which the caller reports to the debugger.
fn take_slot(mask: &mut u8, available: u8) -> Option<u32> {
    let slot = (0..u32::from(available)).find(|slot| *mask & (1u8 << slot) == 0)?;
    *mask |= 1u8 << slot;
    Some(slot)
}

/// Reserve a comparator slot of `kind` on `target` and return it.
fn reserve_slot(target: *mut Target, kind: BreakwatchType) -> Result<u8, TargetError> {
    let reserved = with_priv(target, |priv_| match kind {
        BreakwatchType::BreakHard => {
            take_slot(
                &mut priv_.base.breakpoints_mask,
                priv_.base.breakpoints_available,
            )
        }
        _ => take_slot(
            &mut priv_.base.watchpoints_mask,
            priv_.base.watchpoints_available,
        ),
    });
    match reserved {
        None => Err(TargetError::NotAttached),
        Some(None) => Err(TargetError::NoResources),
        Some(Some(slot)) => Ok(slot as u8),
    }
}

/// Give the comparator slot `slot` of `kind` back, without touching the comparator.
fn release_slot(target: *mut Target, kind: BreakwatchType, slot: u8) {
    let _ = with_priv(target, |priv_| match kind {
        BreakwatchType::BreakHard => priv_.base.breakpoints_mask &= !(1u8 << slot),
        _ => priv_.base.watchpoints_mask &= !(1u8 << slot),
    });
}

/// Program a comparator so it no longer matches anything (the clear path).
fn disarm_comparator(target: *mut Target, kind: BreakwatchType, slot: u8) {
    match kind {
        BreakwatchType::BreakHard => write32(target, cortexm_fpb_comp(u32::from(slot)), 0),
        _ => write32(target, cortexm_dwt_func(u32::from(slot)), 0),
    }
}

/// Program an FPB comparator for a breakpoint (C `cortexm_breakpoint_set()`).
fn breakpoint_set(target: *mut Target, address: u32, slot: u8) -> Result<(), TargetError> {
    let Some(revision) = priv_ref(target).map(|priv_| priv_.flash_patch_revision) else {
        return Err(TargetError::NotAttached);
    };
    let mut value = address;

    /* An ARMv7-M FPB only patches the first 512 MiB and selects which halfword of
     * the word it compares. */
    if revision == 0 {
        value &= CORTEXM_FPB_V1_ADDR_MASK;
        value |= if address & 2 != 0 {
            CORTEXM_FPB_V1_REPLACE_UPPER
        } else {
            CORTEXM_FPB_V1_REPLACE_LOWER
        };
    }
    /* Enable the slot we are about to write. */
    value |= CORTEXM_FPB_COMP_ENABLE;

    write32(target, cortexm_fpb_comp(u32::from(slot)), value);
    Ok(())
}

/// Program a DWT comparator for a watchpoint (C `cortexm_watchpoint_set()`).
fn watchpoint_set(
    target: *mut Target,
    kind: BreakwatchType,
    address: u32,
    size: usize,
    slot: u8,
) -> Result<(), TargetError> {
    if priv_ref(target).is_none() {
        return Err(TargetError::NotAttached);
    }
    let index = u32::from(slot);
    write32(target, cortexm_dwt_comp(index), address);
    if TargetView::new(target).options() & CORTEXM_TOPT_FLAVOUR_V8M != 0 {
        write32(target, cortexm_dwt_func(index), dwtv2_func(kind, size));
    } else {
        write32(target, cortexm_dwt_mask(index), dwt_mask(size));
        write32(target, cortexm_dwt_func(index), dwt_func(target, kind));
    }
    Ok(())
}

/// `TargetDriver::breakwatch_set` for Cortex-M.
///
/// Idempotent per `(kind, address, size)`: GDB re-sends every `Z` packet on each
/// resume, and a second copy would consume a second comparator.
pub(crate) fn breakwatch_set(
    target: *mut Target,
    breakwatch: &mut Breakwatch,
) -> Result<(), TargetError> {
    let kind = breakwatch.kind();
    if !kind.is_hardware() {
        return Err(TargetError::Unsupported);
    }
    let address = breakwatch.addr();
    let size = breakwatch.size();

    let already_armed = with_priv(target, |priv_| {
        priv_
            .breakwatches
            .find(kind, address, size)
            .and_then(|bw| bw.slot())
    })
    .ok_or(TargetError::NotAttached)?;
    if let Some(slot) = already_armed {
        breakwatch.set_slot(slot);
        return Ok(());
    }

    /* Reserve the comparator, then record the entry; a full table gives the slot back. */
    let slot = reserve_slot(target, kind)?;
    let recorded = match with_priv(target, |priv_| {
        priv_
            .breakwatches
            .insert(Breakwatch::armed(kind, address, size, slot))
            .map(|_entry| ())
    }) {
        Some(Ok(_)) => Ok(()),
        Some(Err(err)) => Err(err),
        None => Err(TargetError::NotAttached),
    };
    if let Err(err) = recorded {
        release_slot(target, kind, slot);
        return Err(err);
    }

    match kind {
        BreakwatchType::BreakHard => breakpoint_set(target, address, slot)?,
        _ => watchpoint_set(target, kind, address, size, slot)?,
    }
    breakwatch.set_slot(slot);
    Ok(())
}

/// `TargetDriver::breakwatch_clear` for Cortex-M: `breakwatch` names the entry.
pub(crate) fn breakwatch_clear(
    target: *mut Target,
    breakwatch: &Breakwatch,
) -> Result<(), TargetError> {
    let kind = breakwatch.kind();
    let removed = with_priv(target, |priv_| {
        priv_
            .breakwatches
            .remove(kind, breakwatch.addr(), breakwatch.size())
    });
    let slot = match removed {
        None => return Err(TargetError::NotAttached),
        Some(None) => return Err(TargetError::NotFound),
        Some(Some(entry)) => entry.slot().ok_or(TargetError::InternalError)?,
    };

    release_slot(target, kind, slot);
    disarm_comparator(target, kind, slot);
    Ok(())
}

/// `TargetDriver::breakwatch_clear_all` for Cortex-M: every comparator goes back.
pub(crate) fn breakwatch_clear_all(target: *mut Target) {
    /* Copy the entries out first: releasing a comparator needs the same private state
     * the table lives in. */
    let mut entries = [None; MAX_BREAKWATCHES];
    let count = with_priv(target, |priv_| {
        let mut count = 0;
        for breakwatch in priv_.breakwatches.iter() {
            entries[count] = Some(*breakwatch);
            count += 1;
        }
        priv_.breakwatches.clear();
        count
    })
    .unwrap_or(0);

    for breakwatch in entries.iter().take(count).flatten() {
        let Some(slot) = breakwatch.slot() else {
            continue;
        };
        release_slot(target, breakwatch.kind(), slot);
        disarm_comparator(target, breakwatch.kind(), slot);
    }
}

/// Address of the watchpoint that matched, or 0 (C `cortexm_check_watch()`).
pub(super) fn check_watch(target: *mut Target) -> u32 {
    let Some((available, mask)) = priv_ref(target).map(|priv_| {
        (
            priv_.base.watchpoints_available,
            priv_.base.watchpoints_mask,
        )
    }) else {
        return 0;
    };
    for slot in 0..u32::from(available) {
        if mask & (1u8 << slot) != 0
            && read32(target, cortexm_dwt_func(slot)) & CORTEXM_DWT_FUNC_MATCHED != 0
        {
            return read32(target, cortexm_dwt_comp(slot));
        }
    }
    0
}
