//! RISC-V trigger unit discovery and the per-part policy on top of it.
//!
//! RISC-V cores expose their hardware breakpoint / watchpoint support through the
//! trigger CSRs (`tselect` / `tinfo` / `tdata1`): each trigger slot advertises the
//! trigger types it can implement, and an *execute* breakpoint needs `mcontrol`
//! (`tdata1` type 2) or its successor `mcontrol6` (type 6).
//!
//! This is the Rust port of the C `riscv_hart_discover_triggers()` (see
//! `blackmagic/src/target/riscv_debug.c`), and the result is kept in
//! [`RiscvHart::triggers`] so every RISC-V driver reports the same numbers.
//!
//! Not every part can be asked, and some answers are already known:
//! [`HwTriggers`] lets a driver state what it knows about its part, and only the
//! genuinely unknown cases pay for the CSR scan.

use super::hart::RiscvHart;
use super::rv_csr::{RV_CSR_TDATA1, RV_CSR_TINFO, RV_CSR_TSELECT};

/// Largest number of trigger slots [`discover_triggers`] will inspect.
///
/// Matches C `RV_TRIGGERS_MAX` (`blackmagic/src/target/riscv_debug.h`).
pub const RV_TRIGGERS_MAX: usize = 8;

/// `tselect` written with all ones reads back the index of the last trigger.
pub const RV_TRIG_SELECT_ALL: u32 = 0xffff_ffff;

/// Bits of `tinfo` that are meaningful here: bit 0 marks an empty slot, bits 1..15
/// are "trigger type N is supported". Matches C `RV_TRIGGER_SUPPORT_MASK`.
pub const RV_TRIGGER_SUPPORT_MASK: u32 = 0x0000_fffe;

/// `mcontrol`: address/data match trigger, i.e. what a `Z1` breakpoint needs.
pub const RV_TRIGGER_SUPPORT_MCONTROL: u32 = 1 << 2;

/// `mcontrol6`: the RV32 successor of `mcontrol`, same role.
pub const RV_TRIGGER_SUPPORT_MCONTROL6: u32 = 1 << 6;

/// What the trigger unit of a hart offers.
///
/// `uses[slot]` is the per-slot support mask (C `hart->trigger_uses[]`): zero for a
/// slot the hart does not implement, and [`RV_TRIGGER_SUPPORT_MCONTROL`] when the
/// slot can hold an execute breakpoint. Kept compact on purpose -- this lives in
/// every `RiscvHart`, and the LN builds run out of RAM, not flash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerUnit {
    /// Number of slots the hart claims (0 when it cannot be asked).
    pub slots: u8,
    /// Slots advertising an execute-capable trigger (`mcontrol` / `mcontrol6`).
    pub exec_slots: u8,
    /// Per-slot support mask, indexed by `tselect`.
    pub uses: [u16; RV_TRIGGERS_MAX],
}

impl TriggerUnit {
    /// A hart whose trigger unit was not (or could not be) examined.
    pub const NONE: Self = Self {
        slots: 0,
        exec_slots: 0,
        uses: [0; RV_TRIGGERS_MAX],
    };

    /// True if at least one slot can hold an execute breakpoint.
    pub fn has_exec(&self) -> bool {
        self.exec_slots > 0
    }
}

impl Default for TriggerUnit {
    fn default() -> Self {
        Self::NONE
    }
}

/// Scan the trigger CSRs and record what this hart can do.
///
/// Never fails: an unreachable `tselect` (part without a trigger unit, or a core
/// that only answers it through the program buffer) yields [`TriggerUnit::NONE`],
/// i.e. "no hardware breakpoints", rather than an error.
pub fn discover_triggers(hart: &mut RiscvHart) -> TriggerUnit {
    if hart.csr_write(RV_CSR_TSELECT, RV_TRIG_SELECT_ALL).is_err() {
        return TriggerUnit::NONE;
    }
    // Reading back the all-ones write gives the index of the highest implemented slot.
    let last = match hart.csr_read(RV_CSR_TSELECT) {
        Ok(last) => last,
        Err(_) => return TriggerUnit::NONE,
    };
    let slots = core::cmp::min(last as usize + 1, RV_TRIGGERS_MAX);
    let mut unit = TriggerUnit {
        slots: slots as u8,
        ..TriggerUnit::NONE
    };

    for slot in 0..slots {
        if hart.csr_write(RV_CSR_TSELECT, slot as u32).is_err() {
            continue;
        }
        // `tinfo` is optional; WCH parts do not implement it, and then the trigger
        // type is the top nibble of `tdata1` (RV32).
        let info = match hart.csr_read(RV_CSR_TINFO) {
            Ok(info) if info != 0 => info,
            _ => 1u32 << (hart.csr_read(RV_CSR_TDATA1).unwrap_or(0) >> 28),
        } & RV_TRIGGER_SUPPORT_MASK;

        unit.uses[slot] = info as u16;
        if info & (RV_TRIGGER_SUPPORT_MCONTROL | RV_TRIGGER_SUPPORT_MCONTROL6) != 0 {
            unit.exec_slots += 1;
        }
    }
    unit
}

/// What a driver knows about its part's trigger unit before asking the hart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwTriggers {
    /// The part is known to have no execute trigger (e.g. CH32V003).
    Absent,
    /// The part is known to have execute triggers (e.g. CH32V303/305/307).
    ///
    /// This answer wins over the scan: the scan is still attempted so that the
    /// slot count can be reported and slots can later be allocated, but a part
    /// whose CSRs are not reachable still reports hardware breakpoints.
    Present,
    /// Nothing known: report whatever the hart says.
    Discover,
}

/// The resolved trigger support of a probe: what the driver reports.
///
/// Two bytes on purpose -- the per-slot detail stays in [`RiscvHart::triggers`], and
/// every driver struct lives in `.bss` of builds that are already tight on RAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HwTriggerSupport {
    /// The answer `has_hw_breakpoints()` gives.
    pub has_exec: bool,
    /// Slots the hart advertised as execute-capable (0 if it could not be asked).
    pub exec_slots: u8,
}

impl HwTriggers {
    /// Resolve this policy against `hart`, recording the scan in `hart.triggers`.
    pub fn resolve(self, hart: &mut RiscvHart) -> HwTriggerSupport {
        let unit = match self {
            // Known to have no trigger unit: do not touch a single trigger CSR.
            HwTriggers::Absent => TriggerUnit::NONE,
            HwTriggers::Present | HwTriggers::Discover => discover_triggers(hart),
        };
        hart.triggers = unit;
        HwTriggerSupport {
            // A part known to have triggers keeps that answer even when the scan could
            // not see them (the CSRs may need the program buffer to be reached).
            has_exec: self == HwTriggers::Present || unit.has_exec(),
            exec_slots: unit.exec_slots,
        }
    }
}

// EOF
