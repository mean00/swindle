//! Driver for Nordic Semiconductor nRF54L series targets (RRAM).

use crate::registers::*;
use rsbmp::flash::FlashOperation;
use rsbmp::target::{
    CortexMAccess, MemoryMap, PartDescriptor, RustArmTargetDriver, TOPT_INHIBIT_NRST,
    TOPT_NON_HALTING_MEM_IO,
};

#[derive(Debug, Clone, Copy)]
pub struct Nrf54lDriver {
    ram_len: usize,
    rram_len: usize,
}

pub fn probe(cm: &mut CortexMAccess) -> Option<Nrf54lDriver> {
    if cm.dp_version() < 2 {
        return None;
    }
    if cm.dp_target_partno() != ID_NRF54L {
        return None;
    }

    let info_ram = cm.read32(nrf54l_ficr_reg!(info_ram)) as usize;
    let info_rram = cm.read32(nrf54l_ficr_reg!(info_rram)) as usize;

    Some(Nrf54lDriver {
        ram_len: info_ram * 1024,
        rram_len: info_rram * 1024,
    })
}

impl PartDescriptor for Nrf54lDriver {
    fn name(&self) -> &'static str {
        "nRF54L\0"
    }

    fn part_id(&self) -> u32 {
        ID_NRF54L as u32
    }

    fn target_options(&self) -> u32 {
        TOPT_INHIBIT_NRST | TOPT_NON_HALTING_MEM_IO
    }

    fn memory_map(&self) -> MemoryMap {
        MemoryMap::new()
            .with_flash(NRF54L_RRAM, self.rram_len, 512, 512)
            .with_flash(NRF54L_UICR, 0x1000, 4, 4)
            .with_ram(NRF54L_RAM, self.ram_len)
    }
}

impl RustArmTargetDriver for Nrf54lDriver {
    fn flash_prepare(&mut self, cm: &mut CortexMAccess, _op: FlashOperation) -> bool {
        let writebufsize = 512 / 16;
        cm.write32(
            rramc_reg!(config),
            (writebufsize << 8) | NRF54L_RRAMC_CONFIG_WRITE_ENABLED,
        );
        true
    }

    fn flash_done(&mut self, cm: &mut CortexMAccess) -> bool {
        let mut count = 0;
        while cm.read32(rramc_reg!(bufstatus_writebufempty))
            != NRF54L_RRAMC_BUFSTATUS_WRITEBUFEMPTY_EMPTY
        {
            if cm.check_error() {
                return false;
            }
            count += 1;
            if count > 1_000_000 {
                return false;
            }
        }
        cm.write32(rramc_reg!(config), NRF54L_RRAMC_CONFIG_WRITE_DISABLED);
        true
    }

    fn flash_erase(&mut self, _cm: &mut CortexMAccess, _addr: u32, _len: usize) -> bool {
        // RRAM does not need erase before being written
        true
    }

    fn flash_write(&mut self, cm: &mut CortexMAccess, dest: u32, data: &[u8]) -> bool {
        let mut count = 0;
        while cm.read32(rramc_reg!(readynext)) != NRF54L_RRAMC_READYNEXT_READY {
            if cm.check_error() {
                return false;
            }
            count += 1;
            if count > 1_000_000 {
                return false;
            }
        }
        cm.mem_write(dest, data);
        true
    }

    fn mass_erase(&mut self, cm: &mut CortexMAccess) -> bool {
        cm.write32(
            rramc_reg!(erase_eraseall),
            NRF54L_RRAMC_ERASE_ERASEALL_ERASE,
        );
        let mut count = 0;
        while cm.read32(rramc_reg!(ready)) == NRF54L_RRAMC_READY_BUSY {
            if cm.check_error() {
                return false;
            }
            count += 1;
            if count > 5_000_000 {
                return false;
            }
        }
        true
    }
}
