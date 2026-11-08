//! Hardware breakpoint / watchpoint bookkeeping.
//!
//! The trigger units of a core have a fixed, small number of comparators, so the
//! table that tracks what is armed is a fixed-size `heapless::Vec` and never
//! touches the heap. One [`Breakwatch`] is one armed trigger: what it watches
//! (`kind`), where (`addr`), how wide (`size`) and which comparator slot it owns.
//!
//! The table is owned by the target driver that programs the comparators
//! (Cortex-M: `CortexMPriv`), and the `(kind, addr, size)` triple is the entry's
//! identity - the same key the old C `bw_list` linear search used.

use heapless::Vec;

use super::driver::{BreakwatchType, TargetDriver, TargetError};

/// Ceiling on simultaneously armed hardware triggers: the full comparator space
/// of a Cortex-M core, `CORTEX_MAX_BREAKPOINTS (8) + CORTEX_MAX_WATCHPOINTS (8)`.
pub const MAX_BREAKWATCHES: usize = 16;

const _: () = assert!(
    MAX_BREAKWATCHES
        >= crate::arm::cortexm_registers::CORTEX_MAX_BREAKPOINTS as usize
            + crate::arm::cortexm_registers::CORTEX_MAX_WATCHPOINTS as usize
);

/// One armed trigger and the comparator slot that implements it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Breakwatch {
    kind: BreakwatchType,
    addr: u32,
    size: usize,
    /// Hardware slot once the driver programmed the comparator, `None` before.
    slot: Option<u8>,
}

impl Breakwatch {
    /// An entry that does not own a comparator yet.
    pub const fn new(kind: BreakwatchType, addr: u32, size: usize) -> Self {
        Self {
            kind,
            addr,
            size,
            slot: None,
        }
    }

    /// An entry that already owns a comparator slot.
    pub const fn armed(kind: BreakwatchType, addr: u32, size: usize, slot: u8) -> Self {
        Self {
            kind,
            addr,
            size,
            slot: Some(slot),
        }
    }

    /// What the entry watches.
    pub const fn kind(&self) -> BreakwatchType {
        self.kind
    }

    /// The address the comparator is programmed with.
    pub const fn addr(&self) -> u32 {
        self.addr
    }

    /// Width of the watched region in bytes.
    pub const fn size(&self) -> usize {
        self.size
    }

    /// The comparator slot, once armed.
    pub const fn slot(&self) -> Option<u8> {
        self.slot
    }

    /// Record the comparator slot the driver allocated.
    pub fn set_slot(&mut self, slot: u8) {
        self.slot = Some(slot);
    }

    /// Forget the comparator slot (the entry itself stays in the table).
    pub fn clear_slot(&mut self) {
        self.slot = None;
    }
}

/// The triggers armed on one target. Small, `Copy`-friendly entries; no links,
/// no heap, released with the driver context it lives in.
#[derive(Default)]
pub struct BreakwatchTable {
    entries: Vec<Breakwatch, MAX_BREAKWATCHES>,
}

impl BreakwatchTable {
    /// An empty table, usable in a `const` initialiser.
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Index of the entry with this identity, if it is in the table.
    fn find_index(&self, kind: BreakwatchType, addr: u32, size: usize) -> Option<usize> {
        self.entries
            .iter()
            .position(|bw| bw.kind == kind && bw.addr == addr && bw.size == size)
    }

    /// The entry with this identity, if it is in the table.
    pub fn find(&self, kind: BreakwatchType, addr: u32, size: usize) -> Option<&Breakwatch> {
        self.entries
            .iter()
            .find(|bw| bw.kind == kind && bw.addr == addr && bw.size == size)
    }

    /// The entry with this identity, mutable.
    pub fn find_mut(
        &mut self,
        kind: BreakwatchType,
        addr: u32,
        size: usize,
    ) -> Option<&mut Breakwatch> {
        self.entries
            .iter_mut()
            .find(|bw| bw.kind == kind && bw.addr == addr && bw.size == size)
    }

    /// Record an entry and hand it back for the driver to arm.
    ///
    /// Idempotent: an entry with the same identity is returned as stored instead
    /// of being duplicated (GDB re-sends every `Z` packet on each resume, and a
    /// second copy would consume a second comparator). A full table fails with
    /// [`TargetError::NoResources`].
    pub fn insert(&mut self, breakwatch: Breakwatch) -> Result<&mut Breakwatch, TargetError> {
        let index = match self.find_index(breakwatch.kind, breakwatch.addr, breakwatch.size) {
            Some(index) => index,
            None => {
                self.entries
                    .push(breakwatch)
                    .map_err(|_| TargetError::NoResources)?;
                self.entries.len() - 1
            }
        };
        Ok(&mut self.entries[index])
    }

    /// Unlink an entry and hand it back so its slot can be freed.
    pub fn remove(&mut self, kind: BreakwatchType, addr: u32, size: usize) -> Option<Breakwatch> {
        let index = self.find_index(kind, addr, size)?;
        Some(self.entries.remove(index))
    }

    /// Every armed entry, in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &Breakwatch> {
        self.entries.iter()
    }

    /// Number of entries currently in the table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when nothing is armed.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drop every entry (detach path).
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Arm a hardware trigger on the current target (GDB `Z` path).
///
/// The target's driver owns the table and the comparator, so a repeated request
/// for the same `(kind, addr, size)` is a no-op instead of a second comparator.
pub fn set(kind: BreakwatchType, addr: u32, size: usize) -> Result<(), TargetError> {
    if !kind.is_hardware() {
        return Err(TargetError::Unsupported);
    }
    let mut driver = super::current_driver().ok_or(TargetError::NotAttached)?;
    driver.breakwatch_set(&mut Breakwatch::new(kind, addr, size))
}

/// Disarm the trigger identified by `(kind, addr, size)` on the current target.
pub fn clear(kind: BreakwatchType, addr: u32, size: usize) -> Result<(), TargetError> {
    let mut driver = super::current_driver().ok_or(TargetError::NotAttached)?;
    driver.breakwatch_clear(&Breakwatch::new(kind, addr, size))
}

/// Drop every armed trigger of `target` and release its comparators.
pub fn clear_all_on(target: *mut super::Target) {
    if let Some(mut driver) = super::driver_for(target) {
        driver.breakwatch_clear_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXEC: BreakwatchType = BreakwatchType::BreakHard;
    const WATCH: BreakwatchType = BreakwatchType::WatchWrite;

    #[test]
    fn new_table_is_empty() {
        let table = BreakwatchTable::new();
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
        assert!(table.find(EXEC, 0x100, 2).is_none());
    }

    #[test]
    fn insert_then_find() {
        let mut table = BreakwatchTable::new();
        table.insert(Breakwatch::new(EXEC, 0x8000_1000, 2)).unwrap().set_slot(3);

        assert_eq!(table.len(), 1);
        let found = table.find(EXEC, 0x8000_1000, 2).unwrap();
        assert_eq!(found.slot(), Some(3));
        assert_eq!(found.addr(), 0x8000_1000);
    }

    #[test]
    fn insert_is_idempotent() {
        let mut table = BreakwatchTable::new();
        table.insert(Breakwatch::armed(EXEC, 0x1000, 2, 1)).unwrap();
        table.insert(Breakwatch::new(EXEC, 0x1000, 2)).unwrap();

        assert_eq!(table.len(), 1);
        // The stored entry keeps the slot it already owned.
        assert_eq!(table.find(EXEC, 0x1000, 2).unwrap().slot(), Some(1));
    }

    #[test]
    fn identities_are_distinct() {
        let mut table = BreakwatchTable::new();
        table.insert(Breakwatch::new(EXEC, 0x1000, 2)).unwrap();
        table.insert(Breakwatch::new(EXEC, 0x1000, 4)).unwrap();
        table.insert(Breakwatch::new(WATCH, 0x1000, 2)).unwrap();
        table.insert(Breakwatch::new(WATCH, 0x2000, 2)).unwrap();

        assert_eq!(table.len(), 4);
    }

    #[test]
    fn remove_returns_the_entry_and_frees_it() {
        let mut table = BreakwatchTable::new();
        table.insert(Breakwatch::armed(WATCH, 0x2000, 4, 0)).unwrap();

        let removed = table.remove(WATCH, 0x2000, 4).unwrap();
        assert_eq!(removed.slot(), Some(0));
        assert!(table.is_empty());
        assert!(table.remove(WATCH, 0x2000, 4).is_none());
    }

    #[test]
    fn full_table_fails_without_duplicating() {
        let mut table = BreakwatchTable::new();
        for slot in 0..MAX_BREAKWATCHES {
            table
                .insert(Breakwatch::new(EXEC, 0x1000 + 4 * slot as u32, 2))
                .unwrap();
        }
        assert_eq!(table.len(), MAX_BREAKWATCHES);

        assert_eq!(
            table.insert(Breakwatch::new(EXEC, 0x9000, 2)).err(),
            Some(TargetError::NoResources)
        );
        // A repeat of an armed request still succeeds on a full table.
        assert!(table.insert(Breakwatch::new(EXEC, 0x1000, 2)).is_ok());
        assert_eq!(table.len(), MAX_BREAKWATCHES);
    }

    #[test]
    fn clear_drops_everything() {
        let mut table = BreakwatchTable::new();
        table.insert(Breakwatch::armed(EXEC, 0x1000, 2, 0)).unwrap();
        table.insert(Breakwatch::armed(WATCH, 0x2000, 4, 7)).unwrap();

        table.clear();
        assert!(table.is_empty());
    }
}
