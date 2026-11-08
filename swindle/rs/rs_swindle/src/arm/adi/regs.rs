/*
 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011-2015 Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 * Copyright (C) 2018-2021 Uwe Bonnes <bon@elektron.ikp.physik.tu-darmstadt.de>
 * Copyright (C) 2022-2024 1BitSquared <info@1bitsquared.com>
 * Modified by Rachel Mant <git@dragonmux.network>
 *
 * Rust port of the ARM Debug Interface register definitions and bitfields
 * of blackmagic/src/target/{adiv5,adiv5_internal,adiv6,adi}.h, made for the Swindle firmware. This
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

//! ARM Debug Interface (ADIv5, ADIv6, CoreSight) register map and bitfields.
//!
//! Based on ARM IHI0031 (ADIv5) and ARM IHI0074 (ADIv6).
//! All constants are explicit and free of magic numbers.

// ============================================================================
// DP Registers (Debug Port)
// ============================================================================

pub const ADIV5_LOW_WRITE: u8 = 0;
pub const ADIV5_LOW_READ: u8 = 1;

/// DP Register Addresses (4-byte aligned, 16-bit space)
pub const ADIV5_DP_DPIDR: u16 = 0x0000;
pub const ADIV5_DP_ABORT: u16 = 0x0000;
pub const ADIV5_DP_CTRLSTAT: u16 = 0x0004;
pub const ADIV5_DP_SELECT: u16 = 0x0008;
pub const ADIV5_DP_RESEND: u16 = 0x0008;
pub const ADIV5_DP_RDBUFF: u16 = 0x000c;

/// ADIv5 DP Bank 1+ registers
pub const ADIV5_DP_DLCR: u16 = 0x0004;
pub const ADIV5_DP_TARGETID: u16 = 0x0004;
pub const ADIV5_DP_DLPIDR: u16 = 0x0004;
pub const ADIV5_DP_EVENTSTAT: u16 = 0x0004;
pub const ADIV5_DP_SELECT1: u16 = 0x0004;

pub const ADIV6_DP_DPIDR1: u16 = 0x0000;
pub const ADIV6_DP_BASEPTR0: u16 = 0x0000;
pub const ADIV6_DP_BASEPTR1: u16 = 0x0000;
pub const ADIV6_DP_TARGETID1: u16 = 0x0004;

pub const ADIV5_DP_TARGETSEL: u16 = 0x000c;

/// DP ID Register (`DPIDR`) fields
pub const ADIV5_DP_DPIDR_REVISION_OFFSET: u32 = 28;
pub const ADIV5_DP_DPIDR_REVISION_MASK: u32 = 0xf << ADIV5_DP_DPIDR_REVISION_OFFSET;
pub const ADIV5_DP_DPIDR_PARTNO_OFFSET: u32 = 20;
pub const ADIV5_DP_DPIDR_PARTNO_MASK: u32 = 0xff << ADIV5_DP_DPIDR_PARTNO_OFFSET;
pub const ADIV5_DP_DPIDR_MINDP_OFFSET: u32 = 16;
pub const ADIV5_DP_DPIDR_MINDP: u32 = 1 << ADIV5_DP_DPIDR_MINDP_OFFSET;
pub const ADIV5_DP_DPIDR_VERSION_OFFSET: u32 = 12;
pub const ADIV5_DP_DPIDR_VERSION_MASK: u32 = 0xf << ADIV5_DP_DPIDR_VERSION_OFFSET;
pub const ADIV5_DP_DPIDR_VERSION_DPV1: u32 = 1 << ADIV5_DP_DPIDR_VERSION_OFFSET;
pub const ADIV5_DP_DPIDR_VERSION_DPV2: u32 = 2 << ADIV5_DP_DPIDR_VERSION_OFFSET;
pub const ADIV6_DP_DPIDR_VERSION_DPV3: u32 = 3 << ADIV5_DP_DPIDR_VERSION_OFFSET;
pub const ADIV5_DP_DPIDR_DESIGNER_OFFSET: u32 = 1;
pub const ADIV5_DP_DPIDR_DESIGNER_MASK: u32 = 0x7ff << ADIV5_DP_DPIDR_DESIGNER_OFFSET;

/// DPIDR1 fields (DPv3 Bank 1)
pub const ADIV6_DP_DPIDR1_ASIZE_MASK: u32 = 0x7f;
pub const ADIV6_DP_DPIDR1_ERRMODE: u32 = 1 << 7;
pub const ADIV6_DP_DPIDR1_DESIGNER1_OFFSET: u32 = 8;
pub const ADIV6_DP_DPIDR1_DESIGNER1_MASK: u32 = 0xffff << ADIV6_DP_DPIDR1_DESIGNER1_OFFSET;

/// BASEPTR0 fields (DPv3 Bank 2)
pub const ADIV6_DP_BASEPTR0_VALID: u32 = 1 << 0;
pub const ADIV6_DP_BASE_ADDRESS_MASK: u32 = 0xffff_f000;

/// TARGETID1 fields (DPv3 Bank 6)
pub const ADIV6_DP_TARGETID1_TDESIGNER1_OFFSET: u32 = 8;
pub const ADIV6_DP_TARGETID1_TDESIGNER1_MASK: u32 = 0xffff << ADIV6_DP_TARGETID1_TDESIGNER1_OFFSET;

/// DP `TARGETID` fields
pub const ADIV5_DP_TARGETID_TREVISION_OFFSET: u32 = 28;
pub const ADIV5_DP_TARGETID_TREVISION_MASK: u32 = 0xf << ADIV5_DP_TARGETID_TREVISION_OFFSET;
pub const ADIV5_DP_TARGETID_TPARTNO_OFFSET: u32 = 12;
pub const ADIV5_DP_TARGETID_TPARTNO_MASK: u32 = 0xffff << ADIV5_DP_TARGETID_TPARTNO_OFFSET;
pub const ADIV5_DP_TARGETID_TDESIGNER_OFFSET: u32 = 1;
pub const ADIV5_DP_TARGETID_TDESIGNER_MASK: u32 = 0x7ff << ADIV5_DP_TARGETID_TDESIGNER_OFFSET;

/// DP `TARGETSEL` fields
pub const ADIV5_DP_TARGETSEL_TINSTANCE_OFFSET: u32 = 28;
pub const ADIV5_DP_TARGETSEL_TINSTANCE_MASK: u32 = 0xf << ADIV5_DP_TARGETSEL_TINSTANCE_OFFSET;
pub const ADIV5_DP_TARGETSEL_TPARTNO_OFFSET: u32 = 12;
pub const ADIV5_DP_TARGETSEL_TPARTNO_MASK: u32 = 0xffff << ADIV5_DP_TARGETSEL_TPARTNO_OFFSET;
pub const ADIV5_DP_TARGETSEL_TDESIGNER_OFFSET: u32 = 1;
pub const ADIV5_DP_TARGETSEL_TDESIGNER_MASK: u32 = 0x7ff << ADIV5_DP_TARGETSEL_TDESIGNER_OFFSET;

/// DP `ABORT` register bits
pub const ADIV5_DP_ABORT_ORUNERRCLR: u32 = 1 << 4;
pub const ADIV5_DP_ABORT_WDERRCLR: u32 = 1 << 3;
pub const ADIV5_DP_ABORT_STKERRCLR: u32 = 1 << 2;
pub const ADIV5_DP_ABORT_STKCMPCLR: u32 = 1 << 1;
pub const ADIV5_DP_ABORT_DAPABORT: u32 = 1 << 0;
pub const ADIV5_DP_ABORT_ERRORS: u32 = ADIV5_DP_ABORT_ORUNERRCLR
    | ADIV5_DP_ABORT_WDERRCLR
    | ADIV5_DP_ABORT_STKERRCLR
    | ADIV5_DP_ABORT_STKCMPCLR;

/// DP `CTRL/STAT` register bits
pub const ADIV5_DP_CTRLSTAT_CSYSPWRUPACK: u32 = 1 << 31;
pub const ADIV5_DP_CTRLSTAT_CSYSPWRUPREQ: u32 = 1 << 30;
pub const ADIV5_DP_CTRLSTAT_CDBGPWRUPACK: u32 = 1 << 29;
pub const ADIV5_DP_CTRLSTAT_CDBGPWRUPREQ: u32 = 1 << 28;
pub const ADIV5_DP_CTRLSTAT_CDBGRSTACK: u32 = 1 << 27;
pub const ADIV5_DP_CTRLSTAT_CDBGRSTREQ: u32 = 1 << 26;
pub const ADIV5_DP_CTRLSTAT_WDATAERR: u32 = 1 << 7;
pub const ADIV5_DP_CTRLSTAT_READOK: u32 = 1 << 6;
pub const ADIV5_DP_CTRLSTAT_STICKYERR: u32 = 1 << 5;
pub const ADIV5_DP_CTRLSTAT_STICKYCMP: u32 = 1 << 4;
pub const ADIV5_DP_CTRLSTAT_TRNMODE_MASK: u32 = 3 << 2;
pub const ADIV5_DP_CTRLSTAT_STICKYORUN: u32 = 1 << 1;
pub const ADIV5_DP_CTRLSTAT_ORUNDETECT: u32 = 1 << 0;
pub const ADIV6_DP_CTRLSTAT_ERRMODE: u32 = 1 << 24;

pub const ADIV5_DP_CTRLSTAT_ERRORS: u32 = ADIV5_DP_CTRLSTAT_WDATAERR
    | ADIV5_DP_CTRLSTAT_STICKYERR
    | ADIV5_DP_CTRLSTAT_STICKYCMP
    | ADIV5_DP_CTRLSTAT_STICKYORUN;

/// DP `SELECT` register fields
pub const ADIV5_DP_SELECT_APSEL_OFFSET: u32 = 24;
pub const ADIV5_DP_SELECT_APSEL_MASK: u32 = 0xff << ADIV5_DP_SELECT_APSEL_OFFSET;
pub const ADIV5_DP_SELECT_APBANK_OFFSET: u32 = 4;
pub const ADIV5_DP_SELECT_APBANK_MASK: u32 = 0x0f << ADIV5_DP_SELECT_APBANK_OFFSET;
pub const ADIV5_DP_SELECT_DPBANK_OFFSET: u32 = 0;
pub const ADIV5_DP_SELECT_DPBANK_MASK: u32 = 0x0f << ADIV5_DP_SELECT_DPBANK_OFFSET;

/// DP SELECT register DP bank numbers
pub const ADIV5_DP_BANK0: u32 = 0;
pub const ADIV5_DP_BANK1: u32 = 1;
pub const ADIV5_DP_BANK2: u32 = 2;
pub const ADIV5_DP_BANK3: u32 = 3;
pub const ADIV5_DP_BANK4: u32 = 4;
pub const ADIV5_DP_BANK5: u32 = 5;
pub const ADIV5_DP_BANK6: u32 = 6;

pub const ADIV6_AP_BANK_MASK: u16 = 0x0ff0;

// ============================================================================
// AP Registers (Access Port)
// ============================================================================

pub const ADIV5_APnDP: u16 = 0x1000;
pub const ADIV5_AP_REG_OFFSET: u16 = ADIV5_APnDP;
pub const ADIV5_AP_CSW: u16 = 0x0d00 | ADIV5_AP_REG_OFFSET;
pub const ADIV5_AP_TAR: u16 = 0x0d04 | ADIV5_AP_REG_OFFSET;
pub const ADIV5_AP_DRW: u16 = 0x0d0c | ADIV5_AP_REG_OFFSET;
pub const ADIV5_AP_BD0: u16 = 0x0d10 | ADIV5_AP_REG_OFFSET;
pub const ADIV5_AP_BD1: u16 = 0x0d14 | ADIV5_AP_REG_OFFSET;
pub const ADIV5_AP_BD2: u16 = 0x0d18 | ADIV5_AP_REG_OFFSET;
pub const ADIV5_AP_BD3: u16 = 0x0d1c | ADIV5_AP_REG_OFFSET;
pub const ADIV5_AP_CFG: u16 = 0x0df4 | ADIV5_AP_REG_OFFSET;
pub const ADIV5_AP_BASE: u16 = 0x0df8 | ADIV5_AP_REG_OFFSET;
pub const ADIV5_AP_IDR: u16 = 0x0dfc | ADIV5_AP_REG_OFFSET;

/// AP `CSW` (Control/Status Word) fields
pub const ADIV5_AP_CSW_DBGSWENABLE: u32 = 1 << 31;
pub const ADIV5_AP_CSW_PROT_DEFAULT: u32 = 0x23 << 24;
pub const ADIV5_AP_CSW_SPIDEN: u32 = 1 << 23;
pub const ADIV5_AP_CSW_DEVICEEN: u32 = 1 << 6;
pub const ADIV5_AP_CSW_TRINPROG: u32 = 1 << 7;

pub const ADIV5_AP_CSW_ADDRINC_MASK: u32 = 3 << 4;
pub const ADIV5_AP_CSW_ADDRINC_OFF: u32 = 0 << 4;
pub const ADIV5_AP_CSW_ADDRINC_SINGLE: u32 = 1 << 4;
pub const ADIV5_AP_CSW_ADDRINC_PACKED: u32 = 2 << 4;

pub const ADIV5_AP_CSW_SIZE_MASK: u32 = 7;
pub const ADIV5_AP_CSW_SIZE_BYTE: u32 = 0;
pub const ADIV5_AP_CSW_SIZE_HALFWORD: u32 = 1;
pub const ADIV5_AP_CSW_SIZE_WORD: u32 = 2;

pub const ADIV5_AP_CSW_AHB_MASTERTYPE: u32 = 1 << 29;
pub const ADIV5_AP_CSW_AHB_HPROT_DATA: u32 = 1 << 24;
pub const ADIV5_AP_CSW_AHB_HPROT_PRIV: u32 = 1 << 25;
pub const ADIV5_AP_CSW_AHB_HNONSEC: u32 = 1 << 30;
pub const ADIV5_AP_CSW_AHB_HPROT_MASK: u32 = 0x7f << 24;

/// AP types
pub const ARM_AP_TYPE_JTAG: u8 = 0;
pub const ARM_AP_TYPE_AHB3: u8 = 1;
pub const ARM_AP_TYPE_APB4_5: u8 = 2;
pub const ARM_AP_TYPE_AXI3_4: u8 = 4;
pub const ARM_AP_TYPE_AHB5: u8 = 5;
pub const ARM_AP_TYPE_AHB5_HPROT: u8 = 6;
pub const ARM_AP_TYPE_AXI5: u8 = 7;

/// AP `IDR` fields
pub const ADIV5_AP_IDR_REVISION_OFFSET: u32 = 28;
pub const ADIV5_AP_IDR_REVISION_MASK: u32 = 0xf << ADIV5_AP_IDR_REVISION_OFFSET;
pub const ADIV5_AP_IDR_DESIGNER_OFFSET: u32 = 17;
pub const ADIV5_AP_IDR_DESIGNER_MASK: u32 = 0x7ff << ADIV5_AP_IDR_DESIGNER_OFFSET;
pub const ADIV5_AP_IDR_CLASS_OFFSET: u32 = 13;
pub const ADIV5_AP_IDR_CLASS_MASK: u32 = 0xf << ADIV5_AP_IDR_CLASS_OFFSET;
pub const ADIV5_AP_IDR_TYPE_OFFSET: u32 = 0;
pub const ADIV5_AP_IDR_TYPE_MASK: u32 = 0x0f << ADIV5_AP_IDR_TYPE_OFFSET;

pub const ADIV5_AP_IDR_CLASS_MEM: u8 = 8;
pub const ADIV5_AP_IDR_CLASS_MEMAP: u32 = (ADIV5_AP_IDR_CLASS_MEM as u32) << ADIV5_AP_IDR_CLASS_OFFSET;

// ============================================================================
// CoreSight Identification Registers (PIDR / CIDR / DEVARCH / DEVTYPE)
// ============================================================================

pub const CORESIGHT_ROM_DEVARCH: u16 = 0x0fbc;
pub const CORESIGHT_ROM_DEVID: u16 = 0x0fc8;
pub const CORESIGHT_ROM_DEVTYPE: u16 = 0x0fcc;
/// Class 1 (ROM table) register at the same offset as DEVTYPE.
pub const CORESIGHT_ROM_MEMTYPE: u16 = 0x0fcc;
pub const CORESIGHT_ROM_MEMTYPE_SYSMEM: u32 = 1 << 0;
pub const CORESIGHT_ROM_PIDR4: u16 = 0x0fd0;
pub const CORESIGHT_ROM_PIDR5: u16 = 0x0fd4;
pub const CORESIGHT_ROM_PIDR6: u16 = 0x0fd8;
pub const CORESIGHT_ROM_PIDR7: u16 = 0x0fdc;
pub const CORESIGHT_ROM_PIDR0: u16 = 0x0fe0;
pub const CORESIGHT_ROM_PIDR1: u16 = 0x0fe4;
pub const CORESIGHT_ROM_PIDR2: u16 = 0x0fe8;
pub const CORESIGHT_ROM_PIDR3: u16 = 0x0fec;
pub const CORESIGHT_ROM_CIDR0: u16 = 0x0ff0;
pub const CORESIGHT_ROM_CIDR1: u16 = 0x0ff4;
pub const CORESIGHT_ROM_CIDR2: u16 = 0x0ff8;
pub const CORESIGHT_ROM_CIDR3: u16 = 0x0ffc;

pub const CIDR_PREAMBLE: u32 = 0xb105_000d;
pub const CIDR_CLASS_MASK: u32 = 0x0000_f000;
pub const CIDR_CLASS_SHIFT: u32 = 12;

pub const CID_CLASS_ROM_TABLE: u8 = 0x1;
pub const CID_CLASS_DEBUG_COMPONENT: u8 = 0x9;
pub const CID_CLASS_GENERIC_IP: u8 = 0xe;

pub const DEVARCH_PRESENT: u32 = 1 << 20;
pub const DEVARCH_ARCHID_MASK: u32 = 0xffff;
pub const DEVTYPE_MASK: u32 = 0xff;

// ============================================================================
// Common Protocol Timing & Limits
// ============================================================================

pub const SWD_LINE_RESET_CYCLES: usize = 56;
pub const SWD_JTAG_TO_SWD_MAGIC: u16 = 0xe79e;
pub const SWD_SWD_TO_JTAG_MAGIC: u16 = 0xe73c;
pub const SWD_DORMANT_TO_SWD_MAGIC: [u8; 16] = [
    0x9e, 0xe7, 0x19, 0x60, 0x2f, 0x27, 0x56, 0x82,
    0x9d, 0x52, 0x18, 0xbe, 0xdf, 0xce, 0x55, 0x0e,
];

pub const ADIV5_JTAG_TO_DORMANT_SEQUENCE0: u32 = 0x1f;       // 5 bits
pub const ADIV5_JTAG_TO_DORMANT_SEQUENCE1: u32 = 0x33bbbbba; // 31 bits, LSB
pub const ADIV5_JTAG_TO_DORMANT_SEQUENCE2: u32 = 0xff;       // 8 bits

pub const ADIV5_SELECTION_ALERT_SEQUENCE_0: u32 = 0x6209f392; // 32 bits, LSB
pub const ADIV5_SELECTION_ALERT_SEQUENCE_1: u32 = 0x86852d95; // 32 bits, LSB

// ============================================================================
// Raspberry Pi DPIDR & Multidrop TARGETID Constants
// ============================================================================

/// Raspberry Pi JEP-106 designer code in ARM DPIDR for RP2350.
pub const JEP106_DESIGNER_RPI_RP2350: u16 = 0x929;
/// Part number in ARM DPIDR for RP2350.
pub const DPIDR_PARTNO_RP2350: u16 = 0x0004;
/// Multidrop TARGETID for Raspberry Pi RP2350 (used as fallback when dorm-wakeup TARGETID reads 0).
pub const TARGETID_RP2350: u32 = 0x0004_0929;

/// Raspberry Pi JEP-106 designer code in ARM DPIDR for RP2040.
pub const JEP106_DESIGNER_RPI_RP2040: u16 = 0x927;
/// Part number in ARM DPIDR for RP2040.
pub const DPIDR_PARTNO_RP2040: u16 = 0x0002;
/// Multidrop TARGETID for Raspberry Pi RP2040 (used as fallback when dorm-wakeup TARGETID reads 0).
pub const TARGETID_RP2040: u32 = 0x0100_2927;
pub const ADIV5_SELECTION_ALERT_SEQUENCE_2: u32 = 0xe3ddafe9; // 32 bits, LSB
pub const ADIV5_SELECTION_ALERT_SEQUENCE_3: u32 = 0x19bc0ea2; // 32 bits, LSB

pub const ADIV5_ACTIVATION_CODE_ARM_SWD_DP: u32 = 0x1a; // 8 bits, LSB
pub const ADIV5_ACTIVATION_CODE_ARM_JTAG_DP: u32 = 0x0a; // 8 bits, LSB

pub const ADIV5_JTAG_TO_SWD_SELECT_SEQUENCE: u32 = 0xe79e; // 16 bits, LSB

pub const ADIV5_AP_FLAG_BROADCAST_LANES: u32 = 1 << 0;
/// The AP's ROM table describes system memory (ROM table MEMTYPE.SYSMEM).
pub const ADIV5_AP_FLAG_HAS_MEM: u32 = 1 << 1;

pub const ADIV5_TAR_PAGE_WRAP_MASK: u32 = 0x3ff; // 1024-byte boundary wrap

// CoreSight ROM Table and Component constants
pub const CORESIGHT_ROM_ROMENTRY_PRESENT: u32 = 1 << 0;
pub const CORESIGHT_ROM_ROMENTRY_POWERID_VALID: u32 = 1 << 1;
pub const CORESIGHT_ROM_ROMENTRY_POWERID_SHIFT: u32 = 4;
pub const CORESIGHT_ROM_ROMENTRY_POWERID_MASK: u32 = 0xf << CORESIGHT_ROM_ROMENTRY_POWERID_SHIFT;
pub const CORESIGHT_ROM_ROMENTRY_OFFSET: u32 = 0xffff_f000;

pub const CORESIGHT_ROM_DEVID_FORMAT_MASK: u8 = 0x0f;
pub const CORESIGHT_ROM_DEVID_FORMAT_32BIT: u8 = 0;
pub const CORESIGHT_ROM_DEVID_FORMAT_64BIT: u8 = 1;
pub const CORESIGHT_ROM_DEVID_SYSMEM: u8 = 1 << 0;
pub const CORESIGHT_ROM_DEVID_HAS_POWERREQ: u8 = 1 << 2;

// Power Request Registers in CoreSight ROM Table
pub const CORESIGHT_ROM_PRIDR0: u16 = 0x0e00;
pub const CORESIGHT_ROM_PRIDR0_VERSION_MASK: u8 = 0x0f;
pub const CORESIGHT_ROM_PRIDR0_VERSION_NOT_IMPL: u8 = 0x00;
pub const CORESIGHT_ROM_PRIDR0_HAS_DBG_RESET_REQ: u8 = 1 << 4;
pub const CORESIGHT_ROM_PRIDR0_HAS_SYS_RESET_REQ: u8 = 1 << 5;

pub const CORESIGHT_ROM_DBGRSTRR: u16 = 0x0e10;
pub const CORESIGHT_ROM_DBGRSTAR: u16 = 0x0e14;
pub const CORESIGHT_ROM_DBGRST_REQ: u32 = 1 << 0;

pub const CORESIGHT_ROM_DBGPCR_BASE: u16 = 0x0e20;
pub const CORESIGHT_ROM_DBGPCR_PRESENT: u32 = 1 << 0;
pub const CORESIGHT_ROM_DBGPCR_PWRREQ: u32 = 1 << 1;

pub const CORESIGHT_ROM_DBGPSR_BASE: u16 = 0x0e40;
pub const CORESIGHT_ROM_DBGPSR_STATUS_ON: u32 = 1 << 0;

pub const PIDR_PN_MASK: u64 = 0x0fff;
pub const PIDR_SIZE_MASK: u64 = 0x00f0_0000_0000;

pub const JEP106_MANUFACTURER_ARM: u16 = 0x43b;
pub const JEP106_MANUFACTURER_ARM_CHINA: u16 = 0xa75;

// ADIv6 APv2 Architectures (ARM DEVARCH)
pub const ADIV6_ARCHID_MEM_AP: u16 = 0x0a17;
pub const ADIV6_ARCHID_JTAG_AP: u16 = 0x0a27;
pub const ADIV6_ARCHID_UNKNOWN_AP: u16 = 0x0a47;

// Cortex-M SCS Part Numbers
pub const CORTEXM_PART_SCS_M3: u16 = 0x000;
pub const CORTEXM_PART_SCS_M0: u16 = 0x008;
pub const CORTEXM_PART_SCS_M4: u16 = 0x00c;
pub const CORTEXM_PART_SCS_M23: u16 = 0xd20;
pub const CORTEXM_PART_SCS_M33: u16 = 0xd21;
pub const CORTEXM_PART_SCS_M55: u16 = 0xd22;
pub const CORTEXM_PART_SCS_STAR_MC1: u16 = 0x132;

pub const ARM_ARCH_CORTEX_M_SCS: u16 = 0x2a04;
pub const ARM_ARCH_CORESIGHT_ROM: u16 = 0x0af7;

pub const CORTEXM_DHCSR_ADDR: u32 = 0xe000_edf0;

pub const ALIGN_8BIT: u32 = 0;
pub const ALIGN_16BIT: u32 = 1;
pub const ALIGN_32BIT: u32 = 2;

/// Decode a JEP-106 designer code from DPIDR or TARGETID into Blackmagic internal representation.
/// Bits 10:7 - JEP-106 Continuation code
/// Bits 6:0 - JEP-106 Identity code
pub fn decode_designer(designer: u16) -> u16 {
    ((designer & 0x0780) << 1) | (designer & 0x007f)
}

/// Decode 16-bit extended JEP-106 designer code from DPIDR1 or TARGETID1.
/// Bits 15:7 - JEP-106 Continuation code
/// Bits 6:0  - JEP-106 Identity code
pub fn decode_extended_designer(designer1: u16) -> u16 {
    (((designer1 >> 7) & 0x01ff) << 8) | (designer1 & 0x7f)
}
