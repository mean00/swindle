/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2015 Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2018-2021 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (C) 2022-2024 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the version-independent ADI discovery and ROM table parsing
 * of blackmagic/src/target/adi.c and adiv6.c, made for the Swindle firmware. This
 * file is derived from those C sources and stays under the same license as the original
 * Black Magic code reproduced below.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

//! CoreSight component identification, CIDR/PIDR decoding, and ROM table traversal.

unsafe fn alloc_transport<T>() -> *mut T {
    let ptr = unsafe { crate::session_arena::swindle_session_transport_alloc(core::mem::size_of::<T>()) };
    ptr.cast::<T>()
}

use super::adiv5::{AccessPort, AdiError, DebugPort};
use super::regs::*;
use super::swd::{dormant_to_swd_sequence, jtag_to_swd_sequence, swd_line_reset_sequence, HardwareSwd, SwdTransport};
use crate::arm::cortexm;
use crate::target::{platform_target_clk_output_enable, target_list, target_list_free};

unsafe extern "C" {
    fn swdptap_init();
    fn rp2040_rescue_probe(ap: *mut core::ffi::c_void) -> bool;
}

/// Decode 64-bit PIDR into JEP-106 designer code and part number.
pub fn decode_pidr(pidr: u64) -> (u16, u16) {
    let part_no = (pidr & PIDR_PN_MASK) as u16;
    let continuation = ((pidr >> 32) & 0x0f) as u8;
    let identity = ((pidr >> 12) & 0x7f) as u8;
    let designer = ((continuation as u16) << 8) | (identity as u16);
    (designer, part_no)
}

/// Read 32-bit CIDR from an AP at component base address.
pub fn read_cidr(ap: &mut AccessPort, base_addr: u32) -> Result<u32, AdiError> {
    let mut words = [0u8; 16];
    ap.mem_read_bytes(&mut words, base_addr.wrapping_add(CORESIGHT_ROM_CIDR0 as u32))?;
    let c0 = words[0] as u32;
    let c1 = words[4] as u32;
    let c2 = words[8] as u32;
    let c3 = words[12] as u32;
    Ok(c0 | (c1 << 8) | (c2 << 16) | (c3 << 24))
}

/// Read 64-bit PIDR from an AP at component base address.
pub fn read_pidr(ap: &mut AccessPort, base_addr: u32) -> Result<u64, AdiError> {
    let mut words = [0u8; 32];
    ap.mem_read_bytes(&mut words, base_addr.wrapping_add(CORESIGHT_ROM_PIDR4 as u32))?;
    let pidr4 = words[0] as u64;
    let pidr0 = words[16] as u64;
    let pidr1 = words[20] as u64;
    let pidr2 = words[24] as u64;
    let pidr3 = words[28] as u64;

    Ok((pidr4 << 32) | (pidr3 << 24) | (pidr2 << 16) | (pidr1 << 8) | pidr0)
}

/// Recursively probe a component at `base_addr` on the given AP.
pub fn probe_component(
    ap_ptr: *mut AccessPort,
    base_addr: u32,
    recursion_depth: usize,
) {
    if recursion_depth > 16 || ap_ptr.is_null() {
        return;
    }

    let ap = unsafe { &mut *ap_ptr };
    let Ok(cidr) = read_cidr(ap, base_addr) else {
        return;
    };

    // Validate CIDR preamble (0xb105000d)
    if (cidr & !CIDR_CLASS_MASK) != CIDR_PREAMBLE {
        return;
    }

    let cid_class = ((cidr & CIDR_CLASS_MASK) >> CIDR_CLASS_SHIFT) as u8;
    let Ok(pidr) = read_pidr(ap, base_addr) else {
        return;
    };

    if cid_class == CID_CLASS_ROM_TABLE {
        if (pidr & PIDR_SIZE_MASK) != 0 {
            return;
        }

        // The ROM table's PIDR identifies the device this AP belongs to. SW-DPv1
        // targets have no TARGETID register, so this is the only place the designer
        // and part number can be recovered from (as adi.c's adi_parse_adi_rom_table did).
        if recursion_depth == 0 {
            let (designer, partno) = decode_pidr(pidr);
            ap.designer_code = designer;
            ap.partno = partno;

            if let Ok(memtype) = ap.mem_read32(base_addr.wrapping_add(CORESIGHT_ROM_MEMTYPE as u32)) {
                if (memtype & CORESIGHT_ROM_MEMTYPE_SYSMEM) != 0 {
                    ap.flags |= ADIV5_AP_FLAG_HAS_MEM;
                }
            }
        }

        // Walk ROM table entries (up to 960 entries)
        for i in 0..960u32 {
            let Ok(entry) = ap.mem_read32(base_addr.wrapping_add(i * 4)) else {
                break;
            };
            if entry == 0 {
                break;
            }
            if (entry & CORESIGHT_ROM_ROMENTRY_PRESENT) == 0 {
                continue;
            }
            let offset = entry & CORESIGHT_ROM_ROMENTRY_OFFSET;
            probe_component(ap_ptr, base_addr.wrapping_add(offset), recursion_depth + 1);
        }
    } else {
        let (designer, partno) = decode_pidr(pidr);
        let mut arch_id = 0u16;
        if cid_class == CID_CLASS_DEBUG_COMPONENT || cid_class == CID_CLASS_GENERIC_IP {
            if let Ok(devarch) = ap.mem_read32(base_addr.wrapping_add(CORESIGHT_ROM_DEVARCH as u32)) {
                if (devarch & DEVARCH_PRESENT) != 0 {
                    arch_id = (devarch & DEVARCH_ARCHID_MASK) as u16;
                }
            }
        }
        if designer != JEP106_MANUFACTURER_ARM && designer != JEP106_MANUFACTURER_ARM_CHINA {
            return;
        }

        // Check if this component is a Cortex-M SCS
        let is_cortexm_scs = partno == CORTEXM_PART_SCS_M3
            || partno == CORTEXM_PART_SCS_M0
            || partno == CORTEXM_PART_SCS_M4
            || (arch_id == ARM_ARCH_CORTEX_M_SCS
                && (partno == CORTEXM_PART_SCS_M23
                    || partno == CORTEXM_PART_SCS_M33
                    || partno == CORTEXM_PART_SCS_M55
                    || partno == CORTEXM_PART_SCS_STAR_MC1));

        if is_cortexm_scs {
            cortexm::probe(ap_ptr);
        } else if arch_id == ARM_ARCH_CORESIGHT_ROM {
            // CoreSight component ROM table
            if (pidr & PIDR_SIZE_MASK) == 0 {
                for i in 0..512u32 {
                    let Ok(entry) = ap.mem_read32(base_addr.wrapping_add(i * 4)) else {
                        break;
                    };
                    if (entry & 3) == 0 {
                        break;
                    }
                    if (entry & 3) == 3 {
                        let offset = entry & CORESIGHT_ROM_ROMENTRY_OFFSET;
                        if (entry & CORESIGHT_ROM_ROMENTRY_POWERID_VALID) != 0 {
                            let power_domain = ((entry & CORESIGHT_ROM_ROMENTRY_POWERID_MASK) >> CORESIGHT_ROM_ROMENTRY_POWERID_SHIFT) as u32;
                            let power_reg = power_domain << 2;
                            if let Ok(dbgpcr) = ap.mem_read32(base_addr.wrapping_add(CORESIGHT_ROM_DBGPCR_BASE as u32 + power_reg)) {
                                if (dbgpcr & CORESIGHT_ROM_DBGPCR_PRESENT) != 0 {
                                    let _ = ap.mem_write32(base_addr.wrapping_add(CORESIGHT_ROM_DBGPCR_BASE as u32 + power_reg), CORESIGHT_ROM_DBGPCR_PWRREQ);
                                    for _ in 0..1000 {
                                        if let Ok(dbgpsr) = ap.mem_read32(base_addr.wrapping_add(CORESIGHT_ROM_DBGPSR_BASE as u32 + power_reg)) {
                                            if (dbgpsr & CORESIGHT_ROM_DBGPSR_STATUS_ON) != 0 {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        probe_component(ap_ptr, base_addr.wrapping_add(offset), recursion_depth + 1);
                    }
                }
            }
        }
    }
}

/// Scan an individual DebugPort for APs and attached ARM cores.
pub fn scan_dp(dp_ptr: *mut DebugPort<HardwareSwd>) {
    if dp_ptr.is_null() {
        return;
    }
    let dp = unsafe { &mut *dp_ptr };

    dp.clear_errors();
    if dp.power_cycle_aps().is_err() {
        return;
    }

    let mut invalid_aps = 0usize;
    for apsel in 0..256u16 {
        if invalid_aps >= 8 {
            break;
        }

        let ap_ptr = unsafe {
            let raw = alloc_transport::<AccessPort>();
            if raw.is_null() {
                break;
            }
            raw.write(AccessPort::new(dp_ptr, apsel as u8));
            raw
        };

        let configure_ok = unsafe { (*ap_ptr).configure() }.unwrap_or(false);
        if !configure_ok {
            crate::arm::c_seam::ap_unref(ap_ptr);
            dp.clear_errors();
            invalid_aps += 1;
            continue;
        }

        invalid_aps = 0;
        let ap = unsafe { &mut *ap_ptr };
        let ap_class = ((ap.idr & ADIV5_AP_IDR_CLASS_MASK) >> ADIV5_AP_IDR_CLASS_OFFSET) as u8;

        if ap_class == ADIV5_AP_IDR_CLASS_MEM {
            probe_component(ap_ptr, ap.base, 0);
        }

        // Release the initial discovery reference on the AP
        crate::arm::c_seam::ap_unref(ap_ptr);
    }
}

/// Read 32-bit ID register from an ADIv6 component/AP using DP SELECT bank addressing.
pub fn adiv6_read_id(dp: &mut DebugPort<HardwareSwd>, ap_address: u32, addr: u16) -> Result<u32, AdiError> {
    let mut result = 0u32;
    for i in 0..4u16 {
        let reg = (addr & ADIV6_AP_BANK_MASK) | (i << 2);
        let val = dp.ap_reg_read(0, ap_address, ADIV5_APnDP | reg)?;
        result |= (val & 0xff) << (i * 8);
    }
    Ok(result)
}

/// Read 32-bit CIDR from an ADIv6 component at `ap_address`.
pub fn adiv6_read_cidr(dp: &mut DebugPort<HardwareSwd>, ap_address: u32) -> Result<u32, AdiError> {
    adiv6_read_id(dp, ap_address, CORESIGHT_ROM_CIDR0)
}

/// Read 64-bit PIDR from an ADIv6 component at `ap_address`.
pub fn adiv6_read_pidr(dp: &mut DebugPort<HardwareSwd>, ap_address: u32) -> Result<u64, AdiError> {
    let pidr_upper = adiv6_read_id(dp, ap_address, CORESIGHT_ROM_PIDR4)?;
    let pidr_lower = adiv6_read_id(dp, ap_address, CORESIGHT_ROM_PIDR0)?;
    Ok(((pidr_upper as u64) << 32) | (pidr_lower as u64))
}

/// Parse a CoreSight v0 (Class 0x9) ROM table.
fn adiv6_parse_coresight_v0_rom_table(
    dp_ptr: *mut DebugPort<HardwareSwd>,
    base_address: u32,
    pidr: u64,
    recursion_depth: usize,
) -> bool {
    let dp = unsafe { &mut *dp_ptr };
    let (designer_code, partno) = decode_pidr(pidr);

    let dev_id = (dp.ap_reg_read(0, base_address, ADIV5_APnDP | CORESIGHT_ROM_DEVID).unwrap_or(0) & 0x7f) as u8;
    let rom_format = dev_id & CORESIGHT_ROM_DEVID_FORMAT_MASK;

    let has_pwrctrl = (dev_id & CORESIGHT_ROM_DEVID_HAS_POWERREQ) != 0;
    if has_pwrctrl {
        let pridr0 = (dp.ap_reg_read(0, base_address, ADIV5_APnDP | CORESIGHT_ROM_PRIDR0).unwrap_or(0) & 0x3f) as u8;
        if (pridr0 & CORESIGHT_ROM_PRIDR0_HAS_DBG_RESET_REQ) != 0 {
            let _ = dp.ap_reg_write(0, base_address, ADIV5_APnDP | CORESIGHT_ROM_DBGRSTRR, CORESIGHT_ROM_DBGRST_REQ);
            for _ in 0..1000 {
                let dbgrstar = dp.ap_reg_read(0, base_address, ADIV5_APnDP | CORESIGHT_ROM_DBGRSTAR).unwrap_or(0);
                if (dbgrstar & CORESIGHT_ROM_DBGRST_REQ) != 0 {
                    let _ = dp.ap_reg_write(0, base_address, ADIV5_APnDP | CORESIGHT_ROM_DBGRSTRR, 0);
                    break;
                }
            }
        }
    }

    let max_entries = if rom_format == CORESIGHT_ROM_DEVID_FORMAT_32BIT { 512u32 } else { 256u32 };
    let entry_stride = if rom_format == CORESIGHT_ROM_DEVID_FORMAT_32BIT { 4u16 } else { 8u16 };

    for i in 0..max_entries {
        let entry_offset = (i as u16) * entry_stride;
        let Ok(entry) = dp.ap_reg_read(0, base_address, ADIV5_APnDP | entry_offset) else {
            break;
        };
        let presence = (entry & 0x3) as u8;

        if presence == 0 {
            break;
        }
        if presence == 1 || presence == 2 {
            continue;
        }

        let offset = entry & CORESIGHT_ROM_ROMENTRY_OFFSET;

        if has_pwrctrl && (entry & CORESIGHT_ROM_ROMENTRY_POWERID_VALID) != 0 {
            let power_domain = ((entry & CORESIGHT_ROM_ROMENTRY_POWERID_MASK) >> CORESIGHT_ROM_ROMENTRY_POWERID_SHIFT) as u16;
            let power_reg = power_domain << 2;
            let dbgpcr = dp.ap_reg_read(0, base_address, ADIV5_APnDP | (CORESIGHT_ROM_DBGPCR_BASE + power_reg)).unwrap_or(0);
            if (dbgpcr & CORESIGHT_ROM_DBGPCR_PRESENT) != 0 {
                let _ = dp.ap_reg_write(0, base_address, ADIV5_APnDP | (CORESIGHT_ROM_DBGPCR_BASE + power_reg), CORESIGHT_ROM_DBGPCR_PWRREQ);
                for _ in 0..1000 {
                    let dbgpsr = dp.ap_reg_read(0, base_address, ADIV5_APnDP | (CORESIGHT_ROM_DBGPSR_BASE + power_reg)).unwrap_or(0);
                    if (dbgpsr & CORESIGHT_ROM_DBGPSR_STATUS_ON) != 0 {
                        break;
                    }
                }
            }
        }

        adiv6_component_probe(
            dp_ptr,
            base_address.wrapping_add(offset),
            i,
            designer_code,
            partno,
            recursion_depth + 1,
        );
    }

    true
}

/// Recursively probe an ADIv6 component at `base_address`.
pub fn adiv6_component_probe(
    dp_ptr: *mut DebugPort<HardwareSwd>,
    base_address: u32,
    _entry_number: u32,
    rom_designer_code: u16,
    rom_part_number: u16,
    recursion_depth: usize,
) -> bool {
    if recursion_depth > 16 || dp_ptr.is_null() {
        return false;
    }
    let dp = unsafe { &mut *dp_ptr };

    let Ok(cidr) = adiv6_read_cidr(dp, base_address) else {
        return false;
    };
    if (cidr & !CIDR_CLASS_MASK) != CIDR_PREAMBLE {
        return false;
    }

    let cid_class = ((cidr & CIDR_CLASS_MASK) >> CIDR_CLASS_SHIFT) as u8;
    let Ok(pidr) = adiv6_read_pidr(dp, base_address) else {
        return false;
    };

    if cid_class == CID_CLASS_ROM_TABLE {
        if (pidr & PIDR_SIZE_MASK) != 0 {
            return false;
        }
        for i in 0..960u32 {
            let Ok(entry) = dp.ap_reg_read(0, base_address, ADIV5_APnDP | (i as u16 * 4)) else {
                break;
            };
            if entry == 0 {
                break;
            }
            if (entry & CORESIGHT_ROM_ROMENTRY_PRESENT) == 0 {
                continue;
            }
            let offset = entry & CORESIGHT_ROM_ROMENTRY_OFFSET;
            adiv6_component_probe(dp_ptr, base_address.wrapping_add(offset), i, rom_designer_code, rom_part_number, recursion_depth + 1);
        }
        true
    } else {
        let (designer_code, partno) = decode_pidr(pidr);
        let mut arch_id = 0u16;

        if cid_class == CID_CLASS_DEBUG_COMPONENT {
            if let Ok(devarch) = dp.ap_reg_read(0, base_address, ADIV5_APnDP | CORESIGHT_ROM_DEVARCH) {
                if (devarch & DEVARCH_PRESENT) != 0 {
                    arch_id = (devarch & DEVARCH_ARCHID_MASK) as u16;
                }
            }
        }

        if designer_code == JEP106_MANUFACTURER_ARM
            || designer_code == JEP106_MANUFACTURER_ARM_CHINA
            || arch_id == ARM_ARCH_CORESIGHT_ROM
        {
            if arch_id == ARM_ARCH_CORESIGHT_ROM {
                if (pidr & PIDR_SIZE_MASK) != 0 {
                    return false;
                }
                adiv6_parse_coresight_v0_rom_table(dp_ptr, base_address, pidr, recursion_depth)
            } else if arch_id == ADIV6_ARCHID_MEM_AP || partno == 0x9e2 || partno == 0x9e3 {
                let ap_ptr = unsafe {
                    let raw = alloc_transport::<AccessPort>();
                    if raw.is_null() {
                        return true;
                    }
                    raw.write(AccessPort::new_adiv6(dp_ptr, base_address));
                    raw
                };

                let configure_ok = unsafe { (*ap_ptr).configure() }.unwrap_or(false);
                if !configure_ok {
                    crate::arm::c_seam::ap_unref(ap_ptr);
                    return true;
                }

                let ap = unsafe { &mut *ap_ptr };
                ap.designer_code = rom_designer_code;
                ap.partno = rom_part_number;

                let ap_class = ((ap.idr & ADIV5_AP_IDR_CLASS_MASK) >> ADIV5_AP_IDR_CLASS_OFFSET) as u8;
                if ap_class == ADIV5_AP_IDR_CLASS_MEM {
                    probe_component(ap_ptr, ap.base, 0);
                }

                crate::arm::c_seam::ap_unref(ap_ptr);
                true
            } else {
                true
            }
        } else {
            true
        }
    }
}

/// Initialize and probe an ADIv6 DP (DPv3).
pub fn adiv6_dp_init(dp_ptr: *mut DebugPort<HardwareSwd>) -> bool {
    if dp_ptr.is_null() {
        return false;
    }
    let dp = unsafe { &mut *dp_ptr };

    dp.clear_errors();
    if dp.power_cycle_aps().is_err() {
        return false;
    }

    let _ = dp.select_bank(0, 0, ADIV5_DP_BANK1 as u8);
    let dpidr1 = dp.transport.dp_read(ADIV6_DP_DPIDR1);
    if (dpidr1 & ADIV6_DP_DPIDR1_ERRMODE) != 0 {
        let _ = dp.select_bank(0, 0, 0);
        let ctrlstat = dp.transport.dp_read(ADIV5_DP_CTRLSTAT);
        let _ = dp.dp_write(ADIV5_DP_CTRLSTAT, ctrlstat | ADIV6_DP_CTRLSTAT_ERRMODE);
    }

    let _ = dp.select_bank(0, 0, ADIV5_DP_BANK2 as u8);
    let baseptr0 = dp.transport.dp_read(ADIV6_DP_BASEPTR0);
    let _ = dp.select_bank(0, 0, 0);

    if (baseptr0 & ADIV6_DP_BASEPTR0_VALID) == 0 {
        return false;
    }

    let base_address = baseptr0 & ADIV6_DP_BASE_ADDRESS_MASK;
    adiv6_component_probe(dp_ptr, base_address, 0, dp.designer_code, dp.partno, 0)
}

/// Perform an SWD scan looking for target devices.
pub fn swd_scan_targetid(targetid: u32) -> bool {
    crate::platform::swd::clear_fault();
    unsafe {
        target_list_free();
        swdptap_init();
        platform_target_clk_output_enable(true);
    }

    let mut transport = HardwareSwd;
    dormant_to_swd_sequence(&mut transport);

    let mut dp_targetid = targetid;
    let mut dp_version = 0u8;

    if dp_targetid == 0 {
        let mut dpidr = transport.read_no_check(ADIV5_DP_DPIDR);
        if dpidr == 0 {
            crate::platform::swd::clear_fault();
            dpidr = transport.dp_read(ADIV5_DP_DPIDR);
        }

        if dpidr == 0 {
            crate::platform::swd::clear_fault();
            jtag_to_swd_sequence(&mut transport);
            crate::platform::swd::clear_fault();
            dpidr = transport.read_no_check(ADIV5_DP_DPIDR);
            if dpidr == 0 {
                crate::platform::swd::clear_fault();
                dpidr = transport.dp_read(ADIV5_DP_DPIDR);
            }
        }

        if dpidr == 0 {
            unsafe {
                platform_target_clk_output_enable(false);
            }
            return false;
        }

        dp_version = ((dpidr & ADIV5_DP_DPIDR_VERSION_MASK) >> ADIV5_DP_DPIDR_VERSION_OFFSET) as u8;
        if dp_version >= 2 {
            let mut dp_temp = DebugPort::new(HardwareSwd);
            let _ = dp_temp.select_bank(0, 0, 2);
            dp_targetid = dp_temp.transport.dp_read(ADIV5_DP_TARGETID);
            let _ = dp_temp.select_bank(0, 0, 0);
            if dp_targetid == 0 {
                let designer = decode_designer(((dpidr & ADIV5_DP_DPIDR_DESIGNER_MASK) >> ADIV5_DP_DPIDR_DESIGNER_OFFSET) as u16);
                let partno = ((dpidr & ADIV5_DP_DPIDR_PARTNO_MASK) >> ADIV5_DP_DPIDR_PARTNO_OFFSET) as u16;
                if designer == JEP106_DESIGNER_RPI_RP2350 || partno == DPIDR_PARTNO_RP2350 {
                    dp_targetid = TARGETID_RP2350;
                } else if designer == JEP106_DESIGNER_RPI_RP2040 || partno == DPIDR_PARTNO_RP2040 {
                    dp_targetid = TARGETID_RP2040;
                }
            }
        }
    }

    let scan_multidrop = targetid != 0 || dp_version >= 2;

    if scan_multidrop {
        for instance in 0..16u32 {
            crate::platform::swd::clear_fault();
            swd_line_reset_sequence(&mut transport, true);
            let targetsel = (instance << ADIV5_DP_TARGETSEL_TINSTANCE_OFFSET)
                | (dp_targetid & (ADIV5_DP_TARGETID_TDESIGNER_MASK | ADIV5_DP_TARGETID_TPARTNO_MASK))
                | 1;
            transport.write_no_check(ADIV5_DP_TARGETSEL, targetsel);

            let dpidr = transport.read_no_check(ADIV5_DP_DPIDR);
            if dpidr == 0 {
                continue;
            }

            let dp_ptr = unsafe {
                let raw = alloc_transport::<DebugPort<HardwareSwd>>();
                if raw.is_null() {
                    continue;
                }
                raw.write(DebugPort::new(HardwareSwd));
                raw
            };
            let dp = unsafe { &mut *dp_ptr };
            dp.dev_index = instance as u8;
            dp.version = ((dpidr & ADIV5_DP_DPIDR_VERSION_MASK) >> ADIV5_DP_DPIDR_VERSION_OFFSET) as u8;
            let raw_designer = ((dpidr & ADIV5_DP_DPIDR_DESIGNER_MASK) >> ADIV5_DP_DPIDR_DESIGNER_OFFSET) as u16;
            dp.designer_code = decode_designer(raw_designer);
            dp.partno = ((dpidr & ADIV5_DP_DPIDR_PARTNO_MASK) >> ADIV5_DP_DPIDR_PARTNO_OFFSET) as u16;
            dp.targetsel = targetsel;

            if dp.version >= 2 {
                let _ = dp.select_bank(0, 0, 2);
                let targetid_val = dp.transport.dp_read(ADIV5_DP_TARGETID);
                let _ = dp.select_bank(0, 0, 0);
                let raw_tdesigner = ((targetid_val & ADIV5_DP_TARGETID_TDESIGNER_MASK) >> ADIV5_DP_TARGETID_TDESIGNER_OFFSET) as u16;
                dp.target_designer_code = decode_designer(raw_tdesigner);
                dp.target_partno = ((targetid_val & ADIV5_DP_TARGETID_TPARTNO_MASK) >> ADIV5_DP_TARGETID_TPARTNO_OFFSET) as u16;

                if dp.version >= 3 && raw_tdesigner == 0x07f {
                    let _ = dp.select_bank(0, 0, ADIV5_DP_BANK6 as u8);
                    let targetid1 = dp.transport.dp_read(ADIV6_DP_TARGETID1);
                    let _ = dp.select_bank(0, 0, 0);
                    let tdesigner1 = ((targetid1 & ADIV6_DP_TARGETID1_TDESIGNER1_MASK) >> ADIV6_DP_TARGETID1_TDESIGNER1_OFFSET) as u16;
                    dp.target_designer_code = decode_extended_designer(tdesigner1);
                }
            }

            if dp.version >= 3 && raw_designer == 0x07f {
                let _ = dp.select_bank(0, 0, ADIV5_DP_BANK1 as u8);
                let dpidr1 = dp.transport.dp_read(ADIV6_DP_DPIDR1);
                let _ = dp.select_bank(0, 0, 0);
                let designer1 = ((dpidr1 & ADIV6_DP_DPIDR1_DESIGNER1_MASK) >> ADIV6_DP_DPIDR1_DESIGNER1_OFFSET) as u16;
                dp.designer_code = decode_extended_designer(designer1);
            }

            // RP2040 rescue DP check (instance 1 has partno 0x2)
            if dp.designer_code == crate::target::jep106::RASPBERRY as u16 && dp.partno == 0x2 {
                let _ap_ptr = unsafe {
                    let raw = alloc_transport::<AccessPort>();
                    if !raw.is_null() {
                        raw.write(AccessPort::new(dp_ptr, 0));
                        rp2040_rescue_probe(raw.cast());
                    }
                    raw
                };
                continue;
            }

            if dp.version >= 3 {
                adiv6_dp_init(dp_ptr);
            } else {
                scan_dp(dp_ptr);
            }
        }
    } else {
        let dp_ptr = unsafe {
            let raw = alloc_transport::<DebugPort<HardwareSwd>>();
            if raw.is_null() {
                platform_target_clk_output_enable(false);
                return false;
            }
            raw.write(DebugPort::new(HardwareSwd));
            raw
        };
        let dp = unsafe { &mut *dp_ptr };
        dp.version = dp_version;
        let dpidr = transport.read_no_check(ADIV5_DP_DPIDR);
        if dpidr != 0 {
            let raw_designer = ((dpidr & ADIV5_DP_DPIDR_DESIGNER_MASK) >> ADIV5_DP_DPIDR_DESIGNER_OFFSET) as u16;
            dp.designer_code = decode_designer(raw_designer);
            dp.partno = ((dpidr & ADIV5_DP_DPIDR_PARTNO_MASK) >> ADIV5_DP_DPIDR_PARTNO_OFFSET) as u16;

            if dp.version >= 3 && raw_designer == 0x07f {
                let _ = dp.select_bank(0, 0, ADIV5_DP_BANK1 as u8);
                let dpidr1 = dp.transport.dp_read(ADIV6_DP_DPIDR1);
                let _ = dp.select_bank(0, 0, 0);
                let designer1 = ((dpidr1 & ADIV6_DP_DPIDR1_DESIGNER1_MASK) >> ADIV6_DP_DPIDR1_DESIGNER1_OFFSET) as u16;
                dp.designer_code = decode_extended_designer(designer1);
            }
        }
        if dp.version >= 3 {
            adiv6_dp_init(dp_ptr);
        } else {
            scan_dp(dp_ptr);
        }
    }

    crate::platform::swd::clear_fault();
    let found = unsafe { !target_list.is_null() };
    if !found {
        unsafe {
            platform_target_clk_output_enable(false);
        }
    }
    found
}

/// Exported C ABI symbol for SWD scanning.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn adiv5_swd_scan_targetid(targetid: u32) -> bool {
    swd_scan_targetid(targetid)
}
