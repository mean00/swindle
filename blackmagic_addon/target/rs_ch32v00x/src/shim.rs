#![allow(unsafe_op_in_unsafe_fn)]

use super::driver::Ch32v00xDriver;
use super::registers::*;
use core::ffi::c_char;
use rsbmp::flash::c_adapter::TargetFlash;
use rsbmp::riscv::get_riscv_hart;
use rsbmp::target::{
    target_add_flash_region, target_add_ram32, target_mem_map_free,
    target_set_driver_c, target_set_no_hw_breakpoint_c, target_set_options_c,
    target_set_part_id_c, target_set_sw_breakpoint_helpers_c,
    SwBreakpointHelpersC, TargetC,
};

static mut ACTIVE_DRIVER: Option<Ch32v00xDriver> = None;
static ERASE_SIZES: [usize; 2] = [FLASH_BLOCK_SIZE, 0];

unsafe extern "C" fn ch32v00x_flash_prepare(flash: *mut TargetFlash) -> bool {
    if flash.is_null() {
        return false;
    }
    let target = (*flash).t as *mut TargetC;
    let hart = match get_riscv_hart(target) {
        Some(h) => h,
        None => return false,
    };
    if let Some(ref driver) = ACTIVE_DRIVER {
        driver.prepare_stub(hart)
    } else {
        false
    }
}

unsafe extern "C" fn ch32v00x_flash_erase(flash: *mut TargetFlash, addr: u32, len: usize) -> bool {
    if flash.is_null() {
        return false;
    }
    let target = (*flash).t as *mut TargetC;
    let hart = match get_riscv_hart(target) {
        Some(h) => h,
        None => return false,
    };
    if let Some(ref driver) = ACTIVE_DRIVER {
        driver.erase(hart, addr, len)
    } else {
        false
    }
}

unsafe extern "C" fn ch32v00x_flash_write(
    flash: *mut TargetFlash,
    dest: u32,
    src: *const u8,
    len: usize,
) -> bool {
    if flash.is_null() || src.is_null() {
        return false;
    }
    let target = (*flash).t as *mut TargetC;
    let hart = match get_riscv_hart(target) {
        Some(h) => h,
        None => return false,
    };
    let data = core::slice::from_raw_parts(src, len);
    if let Some(ref driver) = ACTIVE_DRIVER {
        driver.write_stub(hart, dest, data)
    } else {
        false
    }
}

unsafe extern "C" fn ch32v00x_sw_page_size(_t: *mut TargetC) -> u32 {
    if let Some(ref driver) = ACTIVE_DRIVER {
        driver.info.page_size
    } else {
        64
    }
}

unsafe extern "C" fn ch32v00x_sw_page_erase(t: *mut TargetC, addr: u32) -> bool {
    let hart = match get_riscv_hart(t) {
        Some(h) => h,
        None => return false,
    };
    if let Some(ref driver) = ACTIVE_DRIVER {
        driver.page_erase(hart, addr)
    } else {
        false
    }
}

unsafe extern "C" fn ch32v00x_sw_page_write(
    t: *mut TargetC,
    addr: u32,
    src: *const u8,
    page_size: u32,
) -> bool {
    if src.is_null() {
        return false;
    }
    let hart = match get_riscv_hart(t) {
        Some(h) => h,
        None => return false,
    };
    let data = core::slice::from_raw_parts(src, page_size as usize);
    if let Some(ref driver) = ACTIVE_DRIVER {
        driver.page_write(hart, addr, data, page_size)
    } else {
        false
    }
}

static SW_BREAKPOINT_HELPERS: SwBreakpointHelpersC = SwBreakpointHelpersC {
    page_size: ch32v00x_sw_page_size,
    page_erase: ch32v00x_sw_page_erase,
    page_write: ch32v00x_sw_page_write,
};

/// Pure Rust target probe for CH32V00x / CH32V006 microcontrollers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rs_ch32v00x_probe(target: *mut TargetC) -> bool {
    if target.is_null() {
        return false;
    }

    let hart = match get_riscv_hart(target) {
        Some(h) => h,
        None => return false,
    };

    let driver = match Ch32v00xDriver::probe(hart) {
        Some(d) => d,
        None => return false,
    };

    let flash_bytes = (driver.info.flash_size_kb as usize) * 1024;
    let ram_bytes = driver.info.ram_size_kb * 1024;

    ACTIVE_DRIVER = Some(driver);

    target_set_driver_c(target, driver.info.driver_name.as_ptr() as *const c_char);
    target_set_part_id_c(target, driver.info.idcode);
    target_set_options_c(target, TOPT_INHIBIT_NRST);

    target_mem_map_free(target);
    target_add_ram32(target, RAM_ADDRESS, ram_bytes);

    target_add_flash_region(
        target,
        FLASH_ALIAS_ADDRESS,
        flash_bytes,
        FLASH_BLOCK_SIZE,
        FLASH_BLOCK_SIZE,
        ERASE_SIZES.as_ptr(),
        Some(ch32v00x_flash_prepare),
        Some(ch32v00x_flash_erase),
        Some(ch32v00x_flash_write),
    );

    target_add_flash_region(
        target,
        FLASH_PHYSICAL_ADDRESS,
        flash_bytes,
        FLASH_BLOCK_SIZE,
        FLASH_BLOCK_SIZE,
        ERASE_SIZES.as_ptr(),
        Some(ch32v00x_flash_prepare),
        Some(ch32v00x_flash_erase),
        Some(ch32v00x_flash_write),
    );

    target_set_sw_breakpoint_helpers_c(target, &SW_BREAKPOINT_HELPERS);
    target_set_no_hw_breakpoint_c(target, true);

    true
}
