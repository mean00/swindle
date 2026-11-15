/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2022 Vegard Storheil Eriksen <zyp@jvnv.net>
 * Copyright (C) 2024-2025 1BitSquared <info@1bitsquared.com>
 * Written by Vegard Storheil Eriksen <zyp@jvnv.net>
 * Modified by Rachel Mant <git@dragonmux.network>
 * All rights reserved.
 *
 * Rust port of `blackmagic/src/target/nrf91.c` (`nrf91_probe()` and the nRF91 NVMC flash
 * routines), made for the Swindle firmware. This file is derived from that C source and
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
