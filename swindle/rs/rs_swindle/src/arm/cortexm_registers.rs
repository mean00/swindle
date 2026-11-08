//! Cortex-M register map: the `CORTEXM_*` addresses, bit fields and register-file sizes
//! that `super::cortexm` works from.
//!
//! These are the constants of `blackmagic/src/target/cortexm.h`: the System Control Space
//! window and its registers, the FPB and DWT unit bases, the field masks of every register
//! the layer touches, and the sizes of the register-file image. Nothing here does any work;
//! `cortexm.rs` pulls the whole set in with a glob import so the code below reads like the
//! C did.
//!
//! The C names that do not start with `CORTEXM_` live here too, because they describe the
//! same thing and are used from the same place: the CTR fields, the comparator counts, the
//! banked debug-register indices, the CPUID masks and core part numbers of `cortex.h`, and
//! the register-file transfer tables.

/* --- system control space and its registers --- */
pub(crate) const CORTEXM_SCS_BASE: u32 = 0xe000_e000;
pub(crate) const CORTEXM_ICSR: u32 = CORTEXM_SCS_BASE + 0xd04;
pub(crate) const CORTEXM_AIRCR: u32 = CORTEXM_SCS_BASE + 0xd0c;
pub(crate) const CORTEXM_CCR: u32 = CORTEXM_SCS_BASE + 0xd14;
pub(crate) const CORTEXM_CFSR: u32 = CORTEXM_SCS_BASE + 0xd28;
pub(crate) const CORTEXM_HFSR: u32 = CORTEXM_SCS_BASE + 0xd2c;
pub(crate) const CORTEXM_DFSR: u32 = CORTEXM_SCS_BASE + 0xd30;
pub(crate) const CORTEXM_ID_PFR1: u32 = CORTEXM_SCS_BASE + 0xd44;
pub(crate) const CORTEXM_CTR: u32 = CORTEXM_SCS_BASE + 0xd7c;
pub(crate) const CORTEXM_CPACR: u32 = CORTEXM_SCS_BASE + 0xd88;
pub(crate) const CORTEXM_DHCSR: u32 = CORTEXM_SCS_BASE + 0xdf0;
pub(crate) const CORTEXM_DCRSR: u32 = CORTEXM_SCS_BASE + 0xdf4;
pub(crate) const CORTEXM_DCRDR: u32 = CORTEXM_SCS_BASE + 0xdf8;
pub(crate) const CORTEXM_DEMCR: u32 = CORTEXM_SCS_BASE + 0xdfc;

/* cache maintenance */
pub(crate) const CORTEXM_ICIALLU: u32 = CORTEXM_SCS_BASE + 0xf50;
pub(crate) const CORTEXM_DCCMVAC: u32 = CORTEXM_SCS_BASE + 0xf68;
pub(crate) const CORTEXM_DCCIMVAC: u32 = CORTEXM_SCS_BASE + 0xf70;

/* Flash Patch and Breakpoint unit */
pub(crate) const CORTEXM_FPB_BASE: u32 = 0xe000_2000;
pub(crate) const CORTEXM_FPB_CTRL: u32 = CORTEXM_FPB_BASE;
pub(crate) const CORTEXM_FPB_COMP_OFFSET: u32 = 0x008;
pub(crate) const CORTEXM_FPB_COMP_STRIDE: u32 = 4;

/* Data Watchpoint and Trace unit */
pub(crate) const CORTEXM_DWT_BASE: u32 = 0xe000_1000;
pub(crate) const CORTEXM_DWT_CTRL: u32 = CORTEXM_DWT_BASE;
pub(crate) const CORTEXM_DWT_COMP_OFFSET: u32 = 0x020;
pub(crate) const CORTEXM_DWT_MASK_OFFSET: u32 = 0x024;
pub(crate) const CORTEXM_DWT_FUNC_OFFSET: u32 = 0x028;
pub(crate) const CORTEXM_DWT_SLOT_STRIDE: u32 = 0x10;

/* --- bit fields --- */
pub(crate) const CORTEXM_AIRCR_VECTKEY: u32 = 0x05fa << 16;
pub(crate) const CORTEXM_AIRCR_SYSRESETREQ: u32 = 1 << 2;
pub(crate) const CORTEXM_AIRCR_VECTCLRACTIVE: u32 = 1 << 1;
pub(crate) const CORTEXM_CCR_DCACHE_ENABLE: u32 = 1 << 16;
pub(crate) const CORTEXM_CCR_ICACHE_ENABLE: u32 = 1 << 17;
pub(crate) const CORTEXM_HFSR_FORCED: u32 = 1 << 30;
pub(crate) const CORTEXM_DFSR_RESETALL: u32 = 0x1f;
pub(crate) const CORTEXM_DFSR_VCATCH: u32 = 1 << 3;
pub(crate) const CORTEXM_DFSR_DWTTRAP: u32 = 1 << 2;
pub(crate) const CORTEXM_DFSR_BKPT: u32 = 1 << 1;
pub(crate) const CORTEXM_DFSR_HALTED: u32 = 1;
/// ARMv8-M ID_PFR1 Security Extension (TrustZone) bits [7:4] mask.
/// A non-zero value in this 4-bit field indicates TrustZone security extension is implemented.
pub(crate) const CORTEXM_ID_PFR1_SECEXT_IMPL: u32 = 0x0f << 4;
pub(crate) const CORTEXM_DHCSR_DBGKEY: u32 = 0xa05f_0000;
pub(crate) const CORTEXM_DHCSR_S_RESET_ST: u32 = 1 << 25;
pub(crate) const CORTEXM_DHCSR_S_HALT: u32 = 1 << 17;
pub(crate) const CORTEXM_DHCSR_C_MASKINTS: u32 = 1 << 3;
pub(crate) const CORTEXM_DHCSR_C_STEP: u32 = 1 << 2;
pub(crate) const CORTEXM_DHCSR_C_HALT: u32 = 1 << 1;
pub(crate) const CORTEXM_DHCSR_C_DEBUGEN: u32 = 1;
pub(crate) const CORTEXM_DCRSR_REG_WRITE: u32 = 1 << 16;
pub(crate) const CORTEXM_DEMCR_TRCENA: u32 = 1 << 24;
pub(crate) const CORTEXM_DEMCR_VC_HARDERR: u32 = 1 << 10;
pub(crate) const CORTEXM_DEMCR_VC_CORERESET: u32 = 1;
pub(crate) const CORTEXM_FPB_CTRL_REV_MASK: u32 = 0xf;
pub(crate) const CORTEXM_FPB_CTRL_REV_SHIFT: u32 = 28;
pub(crate) const CORTEXM_FPB_CTRL_NUM_CODE_H: u32 = 0x7 << 12;
pub(crate) const CORTEXM_FPB_CTRL_NUM_CODE_L: u32 = 0xf << 4;
pub(crate) const CORTEXM_FPB_CTRL_KEY: u32 = 1 << 1;
pub(crate) const CORTEXM_FPB_CTRL_ENABLE: u32 = 1;
pub(crate) const CORTEXM_FPB_COMP_ENABLE: u32 = 1;
pub(crate) const CORTEXM_FPB_V1_ADDR_MASK: u32 = 0x1fff_fffc;
pub(crate) const CORTEXM_FPB_V1_REPLACE_LOWER: u32 = 0x4000_0000;
pub(crate) const CORTEXM_FPB_V1_REPLACE_UPPER: u32 = 0x8000_0000;
pub(crate) const CORTEXM_DWT_MASK_MAX: u32 = 31;
pub(crate) const CORTEXM_DWT_FUNC_MATCHED: u32 = 1 << 24;
pub(crate) const CORTEXM_DWT_FUNC_DATAVSIZE_WORD: u32 = 2 << 10;
pub(crate) const CORTEXM_DWT_FUNC_FUNC_READ: u32 = 5;
pub(crate) const CORTEXM_DWT_FUNC_FUNC_WRITE: u32 = 6;
pub(crate) const CORTEXM_DWT_FUNC_FUNC_ACCESS: u32 = 7;
pub(crate) const CORTEXM_DWTV2_FUNC_MATCH_READ: u32 = 6;
pub(crate) const CORTEXM_DWTV2_FUNC_MATCH_WRITE: u32 = 5;
pub(crate) const CORTEXM_DWTV2_FUNC_MATCH_ACCESS: u32 = 4;
pub(crate) const CORTEXM_DWTV2_FUNC_ACTION_DEBUG_EVENT: u32 = 1 << 4;
pub(crate) const CORTEXM_DWTV2_FUNC_LEN_SHIFT: u32 = 10;
#[allow(dead_code)]
pub(crate) const CORTEXM_XPSR_THUMB: u32 = 1 << 24;

/* --- register file layout --- */

/// Register file size the C layer installs before TrustZone/FPU are accounted for
/// (`CORTEXM_GENERAL_REG_COUNT` 32-bit registers).
pub(crate) const CORTEXM_GENERAL_REG_COUNT: usize = 20;
/// TrustZone adds the secure/non-secure stack pointer, limit and state registers.
pub(crate) const CORTEXM_TRUSTZONE_REG_COUNT: usize = 10;
/// Size of the register file image C walks: 13 GP + 4 SP + 3 SYS + 10 TrustZone + 33 FPU.
pub(crate) const CORTEXM_MAX_REG_COUNT: usize = 13 + 4 + 3 + 10 + 33;

/// `xPSR` is exposed at GDB register 25 but selected as 16.
pub(crate) const CORTEXM_XPSR_NEW_REGNUM: u32 = 25;
pub(crate) const CORTEXM_XPSR_ORIGINAL_REGNUM: u32 = 16;

/* --- cache type register (CTR) --- */
pub(crate) const CORTEX_CTR_FORMAT_SHIFT: u32 = 29;
pub(crate) const CORTEX_CTR_FORMAT_ARMV7: u32 = 4;
pub(crate) const CORTEX_CTR_ICACHE_LINE_MASK: u32 = 0xf;
pub(crate) const CORTEX_CTR_DCACHE_LINE_SHIFT: u32 = 16;
pub(crate) const CORTEX_CTR_DCACHE_LINE_MASK: u32 = 0xf;

/// FPB code comparators a core may have (C `CORTEX_MAX_BREAKPOINTS`).
pub(crate) const CORTEX_MAX_BREAKPOINTS: u8 = 8;
/// DWT comparators a core may have (C `CORTEX_MAX_WATCHPOINTS`).
pub(crate) const CORTEX_MAX_WATCHPOINTS: u8 = 8;

/// `DB_*` indices of the banked debug registers (C `enum { DB_DHCSR, DB_DCRSR, … }`).
pub(crate) const DB_DCRSR: u32 = 1;
pub(crate) const DB_DCRDR: u32 = 2;

/* --- CPUID: offset in the debug address space, field masks and part numbers --- */

/// Offset of the CPUID register in the core's debug address space (C `CORTEX_CPUID`).
pub(crate) const CORTEX_CPUID: u16 = 0xd00;
/// `CPUID.PARTNO` field mask: the core identification.
pub(crate) const CORTEX_CPUID_PARTNO_MASK: u32 = 0x0000_fff0;
/// `CPUID.REVISION` field mask (`r` of `r<p>`).
pub(crate) const CORTEX_CPUID_REVISION_MASK: u32 = 0x00f0_0000;
/// `CPUID.PATCH` field mask (`p` of `r<p>`).
pub(crate) const CORTEX_CPUID_PATCH_MASK: u32 = 0x0000_000f;

/* Cortex-M CPUIDs (`cortex.h`) */
pub(crate) const CORTEX_M0: u32 = 0xc200;
pub(crate) const CORTEX_M0P: u32 = 0xc600;
pub(crate) const CORTEX_M3: u32 = 0xc230;
pub(crate) const CORTEX_M4: u32 = 0xc240;
pub(crate) const CORTEX_M7: u32 = 0xc270;
pub(crate) const CORTEX_M23: u32 = 0xd200;
pub(crate) const CORTEX_M33: u32 = 0xd210;
pub(crate) const CORTEX_M55: u32 = 0xd220;
pub(crate) const STAR_MC1: u32 = 0x1320;
/* Cortex-R CPUIDs */
pub(crate) const CORTEX_R4: u32 = 0xc140;
pub(crate) const CORTEX_R5: u32 = 0xc150;
/* Cortex-A CPUIDs */
pub(crate) const CORTEX_A5: u32 = 0xc050;
pub(crate) const CORTEX_A7: u32 = 0xc070;
pub(crate) const CORTEX_A8: u32 = 0xc080;
pub(crate) const CORTEX_A9: u32 = 0xc090;
pub(crate) const CORTEX_A55: u32 = 0xd050;

/* --- register file layout as the layer indexes it --- */

/// Register file indices used while unwinding a fault (C `CORTEX_REG_*`).
pub(crate) const CORTEX_REG_SP: usize = 13;
pub(crate) const CORTEX_REG_LR: usize = 14;
pub(crate) const CORTEX_REG_PC: usize = 15;
pub(crate) const CORTEX_REG_MSP: usize = 17;
pub(crate) const CORTEX_REG_PSP: usize = 18;
pub(crate) const CORTEX_REG_SPECIAL: usize = 19;
/// `CONTROL.FPCA` in the packed special register: an extended (FPU) exception frame.
pub(crate) const CORTEX_SPECIAL_FPCA: u32 = 0x0200_0000;
/// `CONTROL.SPSEL`: thread mode was using the process stack pointer.
pub(crate) const CORTEX_SPECIAL_SPSEL: u32 = 0x0400_0000;

/// FPU adds `fpscr` and `s0`-`s31`.
pub(crate) const CORTEX_FLOAT_REG_COUNT: usize = 33;

/* --- register file transfer tables: DCRSR selectors, in transfer order --- */

/// Registers `r0`-`r12`, in DCRSR selector order (C `regnum_cortex_m_gp`).
pub(crate) const REGNUM_CORTEX_M_GP: [u8; 13] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
/// `sp`, `lr`, `pc`, `xpsr` (C `regnum_cortex_m_sp`).
pub(crate) const REGNUM_CORTEX_M_SP: [u8; 4] = [13, 14, 15, 16];
/// `msp`, `psp`, `state` (C `regnum_cortex_m_sys`).
pub(crate) const REGNUM_CORTEX_M_SYS: [u8; 3] = [0x11, 0x12, 0x14];
/// The non-secure and secure stack pointers, limits and states (C
/// `regnum_cortex_m_trustzone`).
pub(crate) const REGNUM_CORTEX_M_TRUSTZONE: [u8; 10] =
    [0x18, 0x19, 0x1e, 0x1f, 0x23, 0x1a, 0x1b, 0x1c, 0x1d, 0x22];
/// `fpscr` and `s0`-`s31` (C `regnum_cortex_mf`).
pub(crate) const REGNUM_CORTEX_MF: [u8; 33] = [
    0x21, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e,
    0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e,
    0x5f,
];
