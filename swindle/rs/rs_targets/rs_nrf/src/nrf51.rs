//! Driver for Nordic Semiconductor nRF51 and nRF52 series targets.

use crate::mon_commands;
use crate::registers::*;
use rsbmp::commands::CommandTree;
use rsbmp::flash::FlashOperation;
use rsbmp::target::{
    CortexMAccess, MemoryMap, PartDescriptor, RustArmTargetDriver, TOPT_INHIBIT_NRST,
    TOPT_NON_HALTING_MEM_IO,
};

#[derive(Debug, Clone, Copy)]
pub struct Nrf51Driver {
    name: &'static str,
    page_size: u32,
    code_size: u32,
    ram_size: usize,
    options: u32,
}

impl Nrf51Driver {
    fn wait_ready(cm: &CortexMAccess) -> bool {
        let mut count = 0;
        while cm.read32(nvmc_reg!(ready)) == 0 {
            if cm.check_error() {
                return false;
            }
            count += 1;
            if count > 1_000_000 {
                return false;
            }
        }
        true
    }
}

pub fn probe(cm: &mut CortexMAccess) -> Option<Nrf51Driver> {
    let page_size = cm.read32(ficr_reg!(codepagesize));
    let code_size = cm.read32(ficr_reg!(codesize));
    if page_size == 0xffff_ffff
        || code_size == 0xffff_ffff
        || page_size == 0
        || code_size == 0
        || page_size > 0x10000
        || code_size > 0x10000
    {
        return None;
    }

    let uid0 = cm.read32(ficr_reg!(deviceid_low));
    let uid1 = cm.read32(ficr_reg!(deviceid_high));
    if uid0 == 0xffff_ffff || uid1 == 0xffff_ffff || uid0 == 0 || uid1 == 0 {
        return None;
    }

    let info_part = cm.read32(ficr_reg!(info_part));
    let (name, ram_size, options) = if info_part != 0xffff_ffff
        && info_part != 0
        && (info_part & 0x00ff_0000) == 0x0052_0000
    {
        let ram_kb = cm.read32(ficr_reg!(info_ram));
        let ram = if ram_kb != 0 && ram_kb != 0xffff_ffff {
            (ram_kb as usize) * 1024
        } else {
            64 * 1024
        };
        ("nRF52\0", ram, TOPT_INHIBIT_NRST | TOPT_NON_HALTING_MEM_IO)
    } else {
        ("nRF51\0", 0x8000, TOPT_INHIBIT_NRST)
    };

    Some(Nrf51Driver {
        name,
        page_size,
        code_size,
        ram_size,
        options,
    })
}

impl PartDescriptor for Nrf51Driver {
    fn name(&self) -> &'static str {
        self.name
    }

    fn part_id(&self) -> u32 {
        0
    }

    fn target_options(&self) -> u32 {
        self.options
    }

    fn memory_map(&self) -> MemoryMap {
        let flash_len = (self.page_size * self.code_size) as usize;
        let page_size = self.page_size as usize;
        let writesize = page_size.min(1024);

        MemoryMap::new()
            .with_flash(0x0000_0000, flash_len, page_size, writesize)
            .with_flash(NRF51_UICR, page_size, page_size, writesize)
            .with_ram(0x2000_0000, self.ram_size)
    }

    fn commands(&self) -> &'static [CommandTree] {
        &mon_commands::COMMANDS
    }
}

impl RustArmTargetDriver for Nrf51Driver {
    fn flash_prepare(&mut self, cm: &mut CortexMAccess, _op: FlashOperation) -> bool {
        // NVMC configuration is set per operation or in write/erase
        cm.write32(nvmc_reg!(config), NRF51_NVMC_CONFIG_WEN);
        Self::wait_ready(cm)
    }

    fn flash_done(&mut self, cm: &mut CortexMAccess) -> bool {
        cm.write32(nvmc_reg!(config), NRF51_NVMC_CONFIG_REN);
        Self::wait_ready(cm)
    }

    fn flash_erase(&mut self, cm: &mut CortexMAccess, addr: u32, _len: usize) -> bool {
        cm.write32(nvmc_reg!(config), NRF51_NVMC_CONFIG_EEN);
        if !Self::wait_ready(cm) {
            return false;
        }

        if addr == NRF51_UICR {
            cm.write32(nvmc_reg!(eraseuicr), 1);
        } else {
            cm.write32(nvmc_reg!(erasepage), addr);
        }

        let ok = Self::wait_ready(cm);
        cm.write32(nvmc_reg!(config), NRF51_NVMC_CONFIG_REN);
        let _ = Self::wait_ready(cm);
        ok
    }

    fn flash_write(&mut self, cm: &mut CortexMAccess, dest: u32, data: &[u8]) -> bool {
        cm.write32(nvmc_reg!(config), NRF51_NVMC_CONFIG_WEN);
        if !Self::wait_ready(cm) {
            return false;
        }

        cm.mem_write(dest, data);
        let ok = Self::wait_ready(cm);
        cm.write32(nvmc_reg!(config), NRF51_NVMC_CONFIG_REN);
        let _ = Self::wait_ready(cm);
        ok
    }

    fn mass_erase(&mut self, cm: &mut CortexMAccess) -> bool {
        cm.write32(nvmc_reg!(config), NRF51_NVMC_CONFIG_EEN);
        if !Self::wait_ready(cm) {
            return false;
        }

        cm.write32(nvmc_reg!(eraseall), 1);
        let ok = Self::wait_ready(cm);
        cm.write32(nvmc_reg!(config), NRF51_NVMC_CONFIG_REN);
        let _ = Self::wait_ready(cm);
        ok
    }
}
