/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2017, 2018 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (c) 2021-2023 Fabrice Prost-Boucle <fabalthazar@falbalab.fr>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the debug-side setup shared by blackmagic/src/target/stm32f1.c
 * (`stm32f1_configure_dbgmcu`), stm32f4.c (`stm32f4_configure_dbgmcu`) and stm32g0.c
 * (`stm32g0_configure_dbgmcu`/`stm32g0_detach`), made for the Swindle firmware. This
 * file is derived from those C sources and stays under the same licenses as the original
 * Black Magic code reproduced in `lib.rs`.
 */

//! Shared core of the STM32 family drivers: what the debugger has to switch on in the
//! part, kept as data instead of as a per-family sequence.
//!
//! Three things vary between the families and nothing else:
//!
//! - the DBGMCU block may sit behind an RCC gate (STM32G0/C0/U0: `RCC_APBENR1.DBGEN`),
//!   in which case it has to be opened before the first DBGMCU write and closed again
//!   after the last one, restoring whatever the part had before;
//! - the freeze bits are spread over one or two registers (`DBGMCU_CR` and
//!   `DBGMCU_APB1FZ` on the F1/F4, `DBGMCU_CR` and `DBGMCU_APBFREEZE1` on the G0);
//! - the bit layouts of those registers.
//!
//! A family therefore builds one [`Stm32Driver`] at probe time with its own name, part id,
//! target options and list of [`FreezeBits`], and forwards `attach()`/`detach()` to it.

use crate::dbgmcu::{clear_bits, set_bits};
use rsbmp::target::CortexMAccess;

/// Number of DBGMCU registers whose freeze bits a driver can set: the control register
/// plus the APB freeze register, on every family this crate serves.
const MAX_FREEZE_REGS: usize = 2;

/// One DBGMCU register and the bits the driver keeps set while attached.
#[derive(Debug, Clone, Copy)]
pub struct FreezeBits {
    /// Address of the DBGMCU register.
    pub addr: u32,
    /// Bits of that register that must stay set (watchdogs, low-power debug).
    pub bits: u32,
}

impl FreezeBits {
    /// A freeze register entry.
    pub const fn new(addr: u32, bits: u32) -> Self {
        Self { addr, bits }
    }

    /// Placeholder of the unused slots of [`Stm32Driver::freeze`].
    const NONE: Self = Self::new(0, 0);
}

/// RCC gate in front of the DBGMCU block (STM32G0/C0/U0 `RCC_APBENR1.DBGEN`).
#[derive(Debug, Clone, Copy)]
pub struct DbgClockGate {
    /// Address of the RCC register carrying the enable bit.
    pub apb_enable: u32,
    /// The enable bit itself.
    pub bit: u32,
}

/// The debug-side state common to every STM32 driver of this workspace.
#[derive(Debug, Clone, Copy)]
pub struct Stm32Driver {
    /// Driver name, NUL terminated for the C `target_s`.
    name: &'static str,
    /// Part id as GDB sees it (`target_s::part_id`).
    part_id: u32,
    /// `TOPT_*` flags of the part.
    options: u32,
    /// Registers whose freeze bits are set while attached.
    freeze: [FreezeBits; MAX_FREEZE_REGS],
    /// Number of valid entries of [`Self::freeze`].
    freeze_len: usize,
    /// RCC gate of the DBGMCU, when the part has one.
    dbg_clock: Option<DbgClockGate>,
}

impl Stm32Driver {
    /// Core of the driver for `name`/`part_id`, with no freeze bits and no clock gate.
    pub const fn new(name: &'static str, part_id: u32, options: u32) -> Self {
        Self {
            name,
            part_id,
            options,
            freeze: [FreezeBits::NONE; MAX_FREEZE_REGS],
            freeze_len: 0,
            dbg_clock: None,
        }
    }

    /// Add a DBGMCU register whose `bits` stay set while attached.
    ///
    /// Extra entries beyond [`MAX_FREEZE_REGS`] are dropped: no part of this workspace
    /// has more than the control and APB freeze registers.
    pub const fn with_freeze(mut self, addr: u32, bits: u32) -> Self {
        if self.freeze_len < MAX_FREEZE_REGS {
            self.freeze[self.freeze_len] = FreezeBits::new(addr, bits);
            self.freeze_len += 1;
        }
        self
    }

    /// Declare the RCC gate of the DBGMCU block.
    pub const fn with_dbg_clock(mut self, apb_enable: u32, bit: u32) -> Self {
        self.dbg_clock = Some(DbgClockGate { apb_enable, bit });
        self
    }

    /// Driver name as the C `target_s` carries it.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Part id as GDB sees it.
    pub const fn part_id(&self) -> u32 {
        self.part_id
    }

    /// `TOPT_*` flags of the part.
    pub const fn options(&self) -> u32 {
        self.options
    }

    /// Open the DBGMCU clock if the part gates it, then set every freeze bit.
    pub fn configure(&self, cm: &CortexMAccess) {
        if let Some(gate) = self.dbg_clock {
            set_bits(cm, gate.apb_enable, gate.bit);
        }
        for entry in &self.freeze[..self.freeze_len] {
            set_bits(cm, entry.addr, entry.bits);
        }
    }

    /// Undo [`Self::configure`]: clear the freeze bits and close the DBGMCU clock again.
    ///
    /// The clock is kept open while the freeze registers are written and the enable bit
    /// is then restored to what the part had before, like the C `stm32g0_detach()`.
    pub fn deconfigure(&self, cm: &CortexMAccess) {
        let Some(gate) = self.dbg_clock else {
            for entry in &self.freeze[..self.freeze_len] {
                clear_bits(cm, entry.addr, entry.bits);
            }
            return;
        };

        let saved = cm.read32(gate.apb_enable) & !gate.bit;
        cm.write32(gate.apb_enable, saved | gate.bit);
        for entry in &self.freeze[..self.freeze_len] {
            clear_bits(cm, entry.addr, entry.bits);
        }
        cm.write32(gate.apb_enable, saved);
    }

    /// Attach hook: attach first, then (re)apply the freeze bits, as detach undid them.
    pub fn attach(&self, cm: &CortexMAccess) -> bool {
        if !cm.cortexm_attach() {
            return false;
        }
        self.configure(cm);
        true
    }

    /// Detach hook: undo [`Self::attach`] and leave the part the way it was found.
    pub fn detach(&self, cm: &CortexMAccess) {
        self.deconfigure(cm);
        cm.cortexm_detach();
    }
}
