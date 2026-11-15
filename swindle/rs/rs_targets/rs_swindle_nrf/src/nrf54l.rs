/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2024 Vegard Storheil Eriksen <zyp@jvnv.net>
 * Copyright (C) 2024-2025 1BitSquared <info@1bitsquared.com>
 * Written by Vegard Storheil Eriksen <zyp@jvnv.net>
 * Modified by Rachel Mant <git@dragonmux.network>
 * All rights reserved.
 *
 * Rust port of `blackmagic/src/target/nrf54l.c` (`nrf54l_probe()` and the nRF54L RRAM
 * controller), made for the Swindle firmware. This file is derived from that C source and
 * stays under the same license as the original Black Magic code reproduced below.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice, this
 *    list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * 3. Neither the name of the copyright holder nor the names of its
 *    contributors may be used to endorse or promote products derived from
 *    this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

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
