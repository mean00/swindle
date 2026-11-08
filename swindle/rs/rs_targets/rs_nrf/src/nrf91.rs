//! Driver for Nordic Semiconductor nRF91 series targets (nRF9160).

use crate::registers::*;
use rsbmp::target::{
    CortexMAccess, MemoryMap, PartDescriptor, RustArmTargetDriver, TOPT_INHIBIT_NRST,
};

#[derive(Debug, Clone, Copy)]
pub struct Nrf91Driver;

impl Nrf91Driver {
    fn wait_ready(cm: &CortexMAccess) -> bool {
        let mut count = 0;
        while cm.read32(nvmc_reg!(NRF91_NVMC, ready)) == 0 {
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

pub fn probe(cm: &mut CortexMAccess) -> Option<Nrf91Driver> {
    if cm.dp_version() < 2 {
        return None;
    }
    if cm.dp_target_partno() != ID_NRF91 {
        return None;
    }

    Some(Nrf91Driver)
}

impl PartDescriptor for Nrf91Driver {
    fn name(&self) -> &'static str {
        "nRF9160\0"
    }

    fn part_id(&self) -> u32 {
        ID_NRF91 as u32
    }

    fn target_options(&self) -> u32 {
        TOPT_INHIBIT_NRST
    }

    fn memory_map(&self) -> MemoryMap {
        MemoryMap::new()
            .with_flash(0x0000_0000, 4096 * 256, 4096, 4096)
            .with_ram(0x2000_0000, 256 * 1024)
    }
}

impl RustArmTargetDriver for Nrf91Driver {
    fn flash_erase(&mut self, cm: &mut CortexMAccess, addr: u32, _len: usize) -> bool {
        cm.write32(nvmc_reg!(NRF91_NVMC, config), NRF91_NVMC_CONFIG_EEN);
        if !Self::wait_ready(cm) {
            return false;
        }

        cm.write32(addr, 0xffff_ffff);
        if !Self::wait_ready(cm) {
            return false;
        }

        cm.write32(nvmc_reg!(NRF91_NVMC, config), NRF91_NVMC_CONFIG_REN);
        Self::wait_ready(cm)
    }

    fn flash_write(&mut self, cm: &mut CortexMAccess, dest: u32, data: &[u8]) -> bool {
        cm.write32(nvmc_reg!(NRF91_NVMC, config), NRF91_NVMC_CONFIG_WEN);
        if !Self::wait_ready(cm) {
            return false;
        }

        cm.mem_write(dest, data);
        if !Self::wait_ready(cm) {
            return false;
        }

        cm.write32(nvmc_reg!(NRF91_NVMC, config), NRF91_NVMC_CONFIG_REN);
        true
    }
}
