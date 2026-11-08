/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (c) 2021-2023 Fabrice Prost-Boucle <fabalthazar@falbalab.fr>
 * Copyright (C) 2022-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the driver half of blackmagic/src/target/stm32g0.c (`stm32g0_probe()`,
 * `stm32g0_attach()`, `stm32g0_detach()` and the `target_flash_s` hooks), made for the
 * Swindle firmware. This file is derived from that C source and stays under the same
 * license as the original Black Magic code reproduced in `lib.rs`.
 */

//! STM32G0/C0/U0 family driver.
//!
//! The probe resolves the part (and, for the parts that report it, the real flash size)
//! once, and the driver then answers from that: the memory map lists the main flash as
//! one contiguous region — even on the dual-bank parts — plus the OTP area, which the
//! flash manager treats as a second, 8-byte-blocked region.

use crate::flash;
use crate::mon_commands;
use crate::parts::{resolve, Part};
use crate::registers::*;
use rs_stm32_common::Stm32Driver;
use rsbmp::gdb_println;
use rsbmp::target::{
    CortexMAccess, MemoryMap, PartDescriptor, RustArmTargetDriver, TOPT_NON_HALTING_MEM_IO,
};

/// Whether irreversible operations (OTP programming, read protection level 2) are allowed.
///
/// The flag is [`G0Driver::irreversible_enabled`]: the `mon` callbacks get no driver handle
/// (the C trampoline passes only the packet text), so they resolve the live driver in the ARM
/// dispatcher. `false` when another driver is attached.
pub fn irreversible_enabled() -> bool {
    rsbmp::arm::driver_registry::active_arm_driver_as::<G0Driver>()
        .is_some_and(|driver| driver.irreversible_enabled)
}

/// Allow or forbid irreversible operations (`mon irreversible enable|disable`).
pub fn set_irreversible_enabled(enabled: bool) {
    // SAFETY: a `mon` command runs outside the driver's own `&mut self` hooks, so the borrow
    // does not alias one (see `active_arm_driver_as_mut`).
    if let Some(driver) =
        unsafe { rsbmp::arm::driver_registry::active_arm_driver_as_mut::<G0Driver>() }
    {
        driver.irreversible_enabled = enabled;
    }
}

/// Part the driver is attached to, or `None` when another driver is.
pub fn active_part() -> Option<&'static Part> {
    rsbmp::arm::driver_registry::active_arm_driver_as::<G0Driver>().map(|driver| driver.part)
}

/// The driver instance for one probed part.
#[derive(Debug, Clone, Copy)]
pub struct G0Driver {
    /// Debug-side core shared with the other STM32 families (name, part id, clock gate).
    common: Stm32Driver,
    /// Part the probe matched.
    part: &'static Part,
    /// Size of the main flash region: the family maximum or the reported size.
    flash_len: usize,
    /// `mon irreversible` state (C `target_s::unsafe_enabled`), off on every probe.
    irreversible_enabled: bool,
}

impl G0Driver {
    /// Driver of `part` whose main flash region is `flash_len` bytes long.
    fn new(part: &'static Part, flash_len: usize) -> Self {
        // The G0-style DBGMCU only answers once `RCC_APBENR1.DBGEN` is set, and the
        // detach path has to close it again.
        let common = Stm32Driver::new(part.name, u32::from(part.id), TOPT_NON_HALTING_MEM_IO)
            .with_dbg_clock(rcc_reg!(apbenr1), RCC_APBENR1_DBGEN)
            .with_freeze(dbgmcu_reg!(cr), DBGMCU_CONFIG_STANDBY | DBGMCU_CONFIG_STOP)
            .with_freeze(
                dbgmcu_reg!(apb1freeze),
                DBGMCU_APBFREEZE1_IWDG | DBGMCU_APBFREEZE1_WWDG,
            );
        Self {
            common,
            part,
            flash_len,
            irreversible_enabled: false,
        }
    }

    /// The part the probe matched.
    pub fn part(&self) -> &'static Part {
        self.part
    }

    /// `(region start, block size)` of the flash region holding `addr`.
    fn region(&self, addr: u32) -> Option<(u32, usize)> {
        let flash_end = FLASH_BANK1_BASE + self.flash_len as u32;
        if (FLASH_BANK1_BASE..flash_end).contains(&addr) {
            Some((FLASH_BANK1_BASE, FLASH_PAGE_SIZE))
        } else if (OTP_BASE..OTP_BASE + OTP_SIZE as u32).contains(&addr) {
            Some((OTP_BASE, OTP_BLOCKSIZE))
        } else {
            None
        }
    }
}

/// Probe an STM32G0/C0/U0 part (C `stm32g0_probe`).
pub fn probe(cm: &mut CortexMAccess) -> Option<G0Driver> {
    let part = resolve(cm)?;
    // The parts with a flash-size register report their real size in KiB; the rest are
    // mapped with their family maximum, so an undersized part can still be programmed.
    let flash_len = if part.flash_size_reg != 0 {
        usize::from(cm.read16(part.flash_size_reg)) * KIB
    } else {
        usize::from(part.flash_max_kib) * KIB
    };

    let driver = G0Driver::new(part, flash_len);
    // Open the DBGMCU clock and freeze the watchdogs before anything else reaches the
    // part, the way the C probe does (it cannot fail: the part id already told us the
    // block is there). `mon irreversible` starts disabled, as in the C probe.
    driver.common.configure(cm);
    Some(driver)
}

/// The shared 256-byte driver cell must hold this driver
/// (`rsbmp::target::DRIVER_STORAGE_SIZE`): a build error here is better than the runtime
/// check `install()` would make at the first probe.
const _: () = assert!(core::mem::size_of::<G0Driver>() <= rsbmp::target::DRIVER_STORAGE_SIZE);

impl PartDescriptor for G0Driver {
    fn name(&self) -> &'static str {
        self.common.name()
    }

    fn part_id(&self) -> u32 {
        self.common.part_id()
    }

    fn target_options(&self) -> u32 {
        self.common.options()
    }

    fn memory_map(&self) -> MemoryMap {
        // Even the dual-bank parts expose one contiguous flash window.
        MemoryMap::new()
            .with_flash(
                FLASH_BANK1_BASE,
                self.flash_len,
                FLASH_PAGE_SIZE,
                FLASH_PAGE_SIZE,
            )
            .with_flash(OTP_BASE, OTP_SIZE, OTP_BLOCKSIZE, OTP_BLOCKSIZE)
            .with_ram(SRAM_BASE, usize::from(self.part.ram_kib) * KIB)
    }

    fn commands(&self) -> &'static [rsbmp::commands::CommandTree] {
        if self.part.reduced_commands {
            &mon_commands::U0_COMMANDS
        } else {
            &mon_commands::COMMANDS
        }
    }
}

impl RustArmTargetDriver for G0Driver {
    fn flash_erase(&mut self, cm: &mut CortexMAccess, addr: u32, _len: usize) -> bool {
        // The flash manager calls this once per block, so only the page holding `addr`
        // has to be erased (same contract as the C driver).
        let Some((start, blocksize)) = self.region(addr) else {
            return false;
        };
        flash::erase_page(cm, start, blocksize, addr)
    }

    fn flash_write(&mut self, cm: &mut CortexMAccess, dest: u32, data: &[u8]) -> bool {
        // Programming the OTP area burns fuses: it needs an explicit opt-in.
        if dest >= OTP_BASE && !self.irreversible_enabled {
            gdb_println!("Irreversible operations disabled");
            flash::op_finish(cm);
            return false;
        }
        flash::program(cm, dest, data)
    }

    fn mass_erase(&mut self, cm: &mut CortexMAccess) -> bool {
        flash::mass_erase(cm)
    }

    fn attach(&mut self, cm: &mut CortexMAccess) -> bool {
        // Attach first, then (re)apply the debug setup: detach undid it.
        self.common.attach(cm)
    }

    fn detach(&mut self, cm: &mut CortexMAccess) {
        self.common.detach(cm);
    }
}
