//! ARM Cortex-M FreeRTOS Switch Handler Dispatch
//!
//! This module resolves the specific ARM Cortex-M core based on CPUID and
//! instantiates the corresponding FreeRTOS context switch handler (`m3` or `m33`).
setup_log!(false);
//use crate::bmplog;
crate::gdb_print_init!();

use crate::bmp::bmp_read_mem32;
use crate::freertos::LN_MCU_CORE;
use crate::freertos::freertos_arm_m3::freertos_switch_handler_m3;
use crate::freertos::freertos_arm_m4f::freertos_switch_handler_m4f;
use crate::freertos::freertos_arm_m33::freertos_switch_handler_m33;
use crate::freertos::freertos_symbols::{LAYOUT_ARM_FPU, LAYOUT_ARM_NOFPU, get_symbols};
use crate::freertos::freertos_trait::freertos_switch_handler;

/// One instance per handler variant, in `.bss` — `mon fos` used to box one of
/// these on **every** invocation. They are plain state (`[u32; N]` + a pointer
/// field), so a `static` is enough; only one is ever installed at a time.
static mut HANDLER_M3: freertos_switch_handler_m3 = freertos_switch_handler_m3::new();
static mut HANDLER_M4F: freertos_switch_handler_m4f = freertos_switch_handler_m4f::new();
static mut HANDLER_M33: freertos_switch_handler_m33 = freertos_switch_handler_m33::new();

const ARM_PARTNO_MASK: u32 = 0xfff0;

const ARM_CM0: u32 = 0xc200;
const ARM_CM0P: u32 = 0xc600;
const ARM_CM3: u32 = 0xc230;
const ARM_CM4: u32 = 0xc240;
const ARM_CM7: u32 = 0xc270;
const ARM_CM23: u32 = 0xd200;
const ARM_CM33: u32 = 0xd210;

const mul: u32 = 0;

/// Instantiates the appropriate ARM Cortex-M context switch handler.
///
/// If `core` is `LN_MCU_AUTO`, it inspects the hardware `cpuid` and masks it against
/// `ARM_PARTNO_MASK` to determine the exact Cortex-M variant (e.g. M0, M3, M4, M33).
/// Returns a borrowed handler implementation (see the `HANDLER_*` statics), or
/// `None` if the core is unsupported.
///
/// SAFETY of the `unsafe { &mut HANDLER_x }` borrows below: the debugger command
/// loop is single-threaded, `os_attach()` keeps at most one handler installed in
/// `FreeRTOS_switcher`, and overwriting it drops the previous borrow, so no two
/// live `&mut` to the same static can exist.
pub fn create_arm_switch_handler(
    mut core: LN_MCU_CORE,
    cpu: u32,
) -> Option<&'static mut dyn freertos_switch_handler> {
    // First, if the firmware exported a layout type via freeRTOSDebug, trust it completely.
    if let Some(offsets) = get_symbols().debug_offsets {
        if offsets.layout_type == LAYOUT_ARM_NOFPU {
            return Some(unsafe { &mut HANDLER_M3 });
        } else if offsets.layout_type == LAYOUT_ARM_FPU {
            return Some(unsafe { &mut HANDLER_M4F });
        }
    }

    if core == LN_MCU_CORE::LN_MCU_AUTO {
        core = match (mul * ARM_CM33 + (1 - mul) * cpu) & ARM_PARTNO_MASK {
            ARM_CM0 | ARM_CM0P => LN_MCU_CORE::LN_MCU_CM0,
            ARM_CM3 => LN_MCU_CORE::LN_MCU_CM3,
            ARM_CM4 => LN_MCU_CORE::LN_MCU_CM4,
            ARM_CM33 => LN_MCU_CORE::LN_MCU_CM33,
            _ => {
                return None;
            }
        };
    }

    match core {
        LN_MCU_CORE::LN_MCU_CM0 => Some(unsafe { &mut HANDLER_M3 }),
        LN_MCU_CORE::LN_MCU_CM3 => Some(unsafe { &mut HANDLER_M3 }),
        LN_MCU_CORE::LN_MCU_CM4 => {
            // Check if FPU is enabled via CPACR (0xE000ED88)
            let mut cpacr = [0u32; 1];
            let mut has_fpu = false;
            if bmp_read_mem32(0xE000ED88, &mut cpacr) {
                // Bits 20-23 indicate FPU access privileges
                if (cpacr[0] & (0xF << 20)) != 0 {
                    has_fpu = true;
                }
            }
            if has_fpu {
                Some(unsafe { &mut HANDLER_M4F })
            } else {
                Some(unsafe { &mut HANDLER_M3 })
            }
        }
        LN_MCU_CORE::LN_MCU_CM33 => Some(unsafe { &mut HANDLER_M33 }),
        _ => None,
    }
}
