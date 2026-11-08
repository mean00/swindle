/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011 Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2017, 2018 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (C) 2023-2025 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the STM32F2/F4/F7 part table, memory map and flash programming
 * routines of blackmagic/src/target/stm32f4.c, made for the Swindle firmware.
 */

//! STM32F2/F4/F7 and GD32F4 family driver.

use crate::flash;
use crate::registers::*;
use rs_stm32_common::Stm32Driver;
use rsbmp::gdb_println;
use rsbmp::target::{
    CortexMAccess, MemoryMap, PartDescriptor, RustArmTargetDriver, TOPT_NON_HALTING_MEM_IO,
};

/// Write chunk handed to the flash manager (`target_flash_s::writesize` in the C driver).
const WRITE_SIZE: usize = KIB;

/// Largest number of flash regions a part of this family can have: the F76x in
/// dual-bank mode lists bank 1, its ITCM alias, bank 2 and bank 2's ITCM alias.
const MAX_REGIONS: usize = 12;

/// Flash size from which a dual-bank part has no single-bank mode (2 MiB).
const DUAL_BANK_ALWAYS_KIB: u16 = 2048;

/// First sector number of each of the three sector groups of a bank.
const GROUP_FIRST_SECTOR: [u8; 3] = [0, 4, 5];
/// First sector number of the three groups of bank 2 (and of the F76x second bank).
const BANK2_FIRST_SECTOR: u8 = 16;

/// One entry of the part table.
#[derive(Debug, Clone, Copy)]
struct Part {
    id: u16,
    /// Driver name, NUL terminated for the C `target_s`.
    name: &'static str,
    /// Largest flash size of the part in KiB (C `max_flashsize`).
    flash_kib: u16,
    /// DTCM RAM of an F7 part in KiB; 0 on the Cortex-M4 parts.
    dtcm_kib: u8,
    /// AHB SRAM of the part in KiB (SRAM1/2 on the F7).
    ram_kib: u16,
    /// The part has the 64 KiB CCM RAM at [`F4_CCM_RAM_BASE`].
    ccm: bool,
    /// The part supports dual-bank mode, selected through the option bytes.
    dual_bank: bool,
    /// Cortex-M7 part: separate ITCM/DTCM memories and an ITCM flash alias.
    f7: bool,
    /// Bank 1 uses 256 KiB sectors above the first 3 × 128 KiB ones (F74x/F76x).
    large_sectors: bool,
}

/// The parts this driver drives (C `stm32f4_get_chip_name` + `stm32f4_attach`).
static PARTS: [Part; 14] = [
    Part { id: PART_STM32F20X, name: "STM32F2\0", flash_kib: 1024, dtcm_kib: 0, ram_kib: 128, ccm: false, dual_bank: false, f7: false, large_sectors: false },
    Part { id: PART_STM32F40X, name: "STM32F40x\0", flash_kib: 1024, dtcm_kib: 0, ram_kib: 128, ccm: true, dual_bank: false, f7: false, large_sectors: false },
    Part { id: PART_STM32F42X, name: "STM32F42x\0", flash_kib: 2048, dtcm_kib: 0, ram_kib: 192, ccm: true, dual_bank: true, f7: false, large_sectors: false },
    Part { id: PART_STM32F446, name: "STM32F446\0", flash_kib: 512, dtcm_kib: 0, ram_kib: 128, ccm: false, dual_bank: false, f7: false, large_sectors: false },
    Part { id: PART_STM32F401C, name: "STM32F401C\0", flash_kib: 256, dtcm_kib: 0, ram_kib: 64, ccm: false, dual_bank: false, f7: false, large_sectors: false },
    Part { id: PART_STM32F411, name: "STM32F411\0", flash_kib: 512, dtcm_kib: 0, ram_kib: 128, ccm: false, dual_bank: false, f7: false, large_sectors: false },
    Part { id: PART_STM32F401E, name: "STM32F401E\0", flash_kib: 512, dtcm_kib: 0, ram_kib: 96, ccm: false, dual_bank: false, f7: false, large_sectors: false },
    Part { id: PART_STM32F46X, name: "STM32F47x\0", flash_kib: 512, dtcm_kib: 0, ram_kib: 320, ccm: true, dual_bank: true, f7: false, large_sectors: false },
    Part { id: PART_STM32F412, name: "STM32F412\0", flash_kib: 1024, dtcm_kib: 0, ram_kib: 256, ccm: false, dual_bank: false, f7: false, large_sectors: false },
    Part { id: PART_STM32F74X, name: "STM32F74x\0", flash_kib: 1024, dtcm_kib: 64, ram_kib: 256, ccm: false, dual_bank: false, f7: true, large_sectors: true },
    Part { id: PART_STM32F76X, name: "STM32F76x\0", flash_kib: 2048, dtcm_kib: 128, ram_kib: 384, ccm: false, dual_bank: true, f7: true, large_sectors: true },
    Part { id: PART_STM32F72X, name: "STM32F72x\0", flash_kib: 512, dtcm_kib: 64, ram_kib: 192, ccm: false, dual_bank: false, f7: true, large_sectors: false },
    Part { id: PART_STM32F410, name: "STM32F410\0", flash_kib: 128, dtcm_kib: 0, ram_kib: 32, ccm: false, dual_bank: false, f7: false, large_sectors: false },
    Part { id: PART_STM32F413, name: "STM32F413\0", flash_kib: 1536, dtcm_kib: 0, ram_kib: 320, ccm: false, dual_bank: false, f7: false, large_sectors: false },
];

/// Everything the `mon` commands of this family need to know about the part.
#[derive(Debug, Clone, Copy)]
pub struct F4Info {
    pub part_id: u16,
    /// Bus width of the program steps, changed by `mon psize`.
    pub psize: u8,
}

/// State of the part the driver is attached to, or `None` when another driver is.
pub fn active_info() -> Option<F4Info> {
    rsbmp::arm::driver_registry::active_arm_driver_as::<F4Driver>().map(|driver| F4Info {
        part_id: driver.part.id,
        psize: driver.psize,
    })
}

/// Change the program step width (`mon psize`).
pub fn set_psize(psize: u8) {
    if let Some(driver) =
        unsafe { rsbmp::arm::driver_registry::active_arm_driver_as_mut::<F4Driver>() }
    {
        driver.psize = psize;
    }
}

/// One flash region of the part.
#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub start: u32,
    pub length: usize,
    pub blocksize: usize,
    /// Sector number of the block at [`Region::start`].
    pub base_sector: u8,
}

/// Sector sizes of one bank.
const fn sector_sizes(large: bool) -> (usize, usize, usize, usize) {
    if large {
        (0x2_0000, 0x8000, 0x2_0000, 0x4_0000)
    } else {
        (0x1_0000, 0x4000, 0x1_0000, 0x2_0000)
    }
}

/// The `group`-th sector group of a bank of `bank_length` bytes.
fn bank_group(group: usize, bank_length: usize, large: bool) -> Option<(usize, usize, usize)> {
    let (small_span, small_block, mid_block, big_block) = sector_sizes(large);
    match group {
        0 => Some((0, small_span, small_block)),
        1 if bank_length > small_span => Some((small_span, mid_block, mid_block)),
        2 => {
            let remaining = bank_length.saturating_sub(small_span + mid_block);
            (remaining != 0).then_some((small_span + mid_block, remaining, big_block))
        }
        _ => None,
    }
}

/// The flash map of the GD32F4 parts.
static GD32F4_REGIONS: [Region; 7] = [
    Region { start: 0x0800_0000, length: 0x1_0000, blocksize: 0x4000, base_sector: 0 },
    Region { start: 0x0801_0000, length: 0x1_0000, blocksize: 0x1_0000, base_sector: 4 },
    Region { start: 0x0802_0000, length: 0xe_0000, blocksize: 0x2_0000, base_sector: 5 },
    Region { start: 0x0810_0000, length: 0x1_0000, blocksize: 0x4000, base_sector: 16 },
    Region { start: 0x0811_0000, length: 0x1_0000, blocksize: 0x1_0000, base_sector: 20 },
    Region { start: 0x0812_0000, length: 0xe_0000, blocksize: 0x2_0000, base_sector: 21 },
    Region { start: 0x0820_0000, length: 0x10_0000, blocksize: 0x4_0000, base_sector: 12 },
];

/// The driver instance for one probed part.
#[derive(Debug, Clone, Copy)]
pub struct F4Driver {
    part: &'static Part,
    use_dual_bank: bool,
    gd32: bool,
    psize: u8,
    common: Stm32Driver,
}

impl F4Driver {
    fn new(part: &'static Part, use_dual_bank: bool, gd32: bool, options: u32) -> Self {
        let common = Stm32Driver::new(part.name, u32::from(part.id), options)
            .with_freeze(
                F4_DBGMCU_CTRL,
                F4_DBGMCU_CTRL_DBG_SLEEP | F4_DBGMCU_CTRL_DBG_STANDBY | F4_DBGMCU_CTRL_DBG_STOP,
            )
            .with_freeze(
                F4_DBGMCU_APB1FREEZE,
                F4_DBGMCU_APB1FREEZE_WWDG | F4_DBGMCU_APB1FREEZE_IWDG,
            );
        Self {
            part,
            use_dual_bank,
            gd32,
            psize: flash::DEFAULT_PSIZE,
            common,
        }
    }

    fn bank_length(&self) -> usize {
        let flash_len = usize::from(self.part.flash_kib) * KIB;
        if self.use_dual_bank {
            flash_len / 2
        } else {
            flash_len
        }
    }

    fn region_at(&self, index: usize) -> Option<Region> {
        if self.gd32 {
            return GD32F4_REGIONS.get(index).copied();
        }

        let bank_length = self.bank_length();
        let bank_length_u32 = bank_length as u32;
        let sources = [
            (FLASH_BANK1_BASE, 0u8, self.part.large_sectors),
            (F7_ITCM_FLASH_BASE, 0u8, self.part.large_sectors),
            (FLASH_BANK1_BASE + bank_length_u32, BANK2_FIRST_SECTOR, false),
            (
                F7_ITCM_FLASH_BASE + bank_length_u32,
                BANK2_FIRST_SECTOR,
                false,
            ),
        ];
        let enabled = [
            true,
            self.part.f7,
            self.use_dual_bank,
            self.use_dual_bank && self.part.f7,
        ];

        let mut slot = 0;
        for (source, (start, base_sector, large)) in sources.iter().enumerate() {
            if !enabled[source] {
                continue;
            }
            for group in 0..GROUP_FIRST_SECTOR.len() {
                let Some((offset, length, blocksize)) = bank_group(group, bank_length, *large) else {
                    break;
                };
                if slot == index {
                    return Some(Region {
                        start: start + offset as u32,
                        length,
                        blocksize,
                        base_sector: base_sector + GROUP_FIRST_SECTOR[group],
                    });
                }
                slot += 1;
            }
        }
        None
    }

    fn region_for(&self, addr: u32) -> Option<Region> {
        (0..MAX_REGIONS)
            .filter_map(|index| self.region_at(index))
            .find(|region| addr >= region.start && addr - region.start < region.length as u32)
    }

    fn finish(self, cm: &mut CortexMAccess) -> Self {
        self.common.configure(cm);
        self
    }
}

static GD32F4_PARTS: [Part; 3] = [
    Part { id: PART_GD32F450, name: "GD32F450\0", flash_kib: 0, dtcm_kib: 0, ram_kib: 320, ccm: true, dual_bank: false, f7: false, large_sectors: false },
    Part { id: PART_GD32F470, name: "GD32F470\0", flash_kib: 0, dtcm_kib: 0, ram_kib: 320, ccm: true, dual_bank: false, f7: false, large_sectors: false },
    Part { id: PART_GD32F405, name: "GD32F405\0", flash_kib: 0, dtcm_kib: 0, ram_kib: 320, ccm: true, dual_bank: false, f7: false, large_sectors: false },
];

fn part_by_id(part_id: u16) -> Option<&'static Part> {
    PARTS.iter().find(|part| part.id == part_id)
}

fn read_idcode(cm: &CortexMAccess) -> u16 {
    let idcode = (cm.read32(F4_DBGMCU_IDCODE) & IDCODE_DEV_ID_MASK) as u16;
    if idcode == PART_STM32F20X && cm.cpuid() & CORTEX_CPUID_PARTNO_MASK == CORTEX_M4 {
        return PART_STM32F40X;
    }
    idcode
}

fn dual_bank_enabled(cm: &CortexMAccess, part: &Part) -> bool {
    let optcr = cm.read32(F4_FPEC_OPTION_CTRL);
    if part.f7 {
        optcr & F4_FPEC_OPTION_CTRL_NDBANK == 0
    } else if part.flash_kib < DUAL_BANK_ALWAYS_KIB {
        optcr & F4_FPEC_OPTION_CTRL_DB1M == 0
    } else {
        true
    }
}

/// Probe an STM32F2/F4/F7 part (C `stm32f4_probe`).
pub fn probe_stm32(cm: &mut CortexMAccess) -> Option<F4Driver> {
    let part = part_by_id(read_idcode(cm))?;
    let use_dual_bank = part.dual_bank && dual_bank_enabled(cm, part);
    let options = if part.f7 || part.id == PART_STM32F20X {
        0
    } else {
        TOPT_NON_HALTING_MEM_IO
    };
    Some(
        F4Driver::new(part, use_dual_bank, false, options).finish(cm),
    )
}

/// Probe a GD32F4 part (C `gd32f4_probe`).
pub fn probe_gd32(cm: &mut CortexMAccess) -> Option<F4Driver> {
    let part_id = read_idcode(cm);
    let Some(part) = GD32F4_PARTS.iter().find(|part| part.id == part_id) else {
        return None;
    };
    Some(
        F4Driver::new(part, false, true, TOPT_NON_HALTING_MEM_IO).finish(cm),
    )
}

const _: () = assert!(core::mem::size_of::<F4Driver>() <= rsbmp::target::DRIVER_STORAGE_SIZE);

impl PartDescriptor for F4Driver {
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
        let mut map = MemoryMap::new();
        let mut index = 0;
        while let Some(region) = self.region_at(index) {
            map = map.with_flash(region.start, region.length, region.blocksize, WRITE_SIZE);
            index += 1;
        }

        let ram_len = usize::from(self.part.ram_kib) * KIB;
        if self.part.f7 {
            let dtcm_len = usize::from(self.part.dtcm_kib) * KIB;
            map = map
                .with_ram(F7_ITCM_RAM_BASE, F7_ITCM_RAM_LEN)
                .with_ram(SRAM_BASE, dtcm_len)
                .with_ram(SRAM_BASE + dtcm_len as u32, ram_len);
        } else {
            if self.part.ccm {
                map = map.with_ram(F4_CCM_RAM_BASE, F4_CCM_RAM_LEN);
            }
            map = map.with_ram(SRAM_BASE, ram_len);
        }
        map
    }

    fn commands(&self) -> &'static [rsbmp::commands::CommandTree] {
        &crate::mon_commands::COMMANDS
    }
}

impl RustArmTargetDriver for F4Driver {
    fn flash_erase(&mut self, cm: &mut CortexMAccess, addr: u32, _len: usize) -> bool {
        let Some(region) = self.region_for(addr) else {
            gdb_println!("stm32f4: no flash region at 0x", Hex(addr));
            return false;
        };
        let sector = usize::from(region.base_sector) + (addr - region.start) as usize / region.blocksize;
        flash::erase_sector(cm, sector as u8, self.psize)
    }

    fn flash_write(&mut self, cm: &mut CortexMAccess, dest: u32, data: &[u8]) -> bool {
        let dest = if (F7_ITCM_FLASH_BASE..FLASH_BANK1_BASE).contains(&dest) {
            dest + (FLASH_BANK1_BASE - F7_ITCM_FLASH_BASE)
        } else {
            dest
        };
        flash::program(cm, dest, data, self.psize)
    }

    fn mass_erase(&mut self, cm: &mut CortexMAccess) -> bool {
        flash::mass_erase(cm, self.use_dual_bank || self.gd32)
    }

    fn attach(&mut self, cm: &mut CortexMAccess) -> bool {
        self.common.attach(cm)
    }

    fn detach(&mut self, cm: &mut CortexMAccess) {
        self.common.detach(cm);
    }
}
