//! Central C-to-Rust Target Driver Shim.
//!
//! This is the SINGLE central integration point that adapts any pure Rust
//! target driver implementing `RustTargetDriver` into the BlackMagic C/C++ target subsystem.
//! Individual target drivers (such as `rs_ch32v00x`) do not implement their own shims.

#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::c_char;
use rs_ch32v00x::Ch32v00xDriver;
use rs_ch32v123xx::Ch32v123xxDriver;
use rsbmp::flash::c_adapter::TargetFlash;
use rsbmp::riscv::get_riscv_hart;
use rsbmp::target::{
    target_add_default_commands_c, target_add_flash_region, target_add_ram32, target_mem_map_free,
    target_set_crc32_c, target_set_driver_c, target_set_no_hw_breakpoint_c, target_set_options_c,
    target_set_part_id_c, target_set_sw_breakpoint_helpers_c, FlashDesc, RamDesc, RustTargetDriver,
    SwBreakpointHelpersC, TargetC,
};

/// Enumeration of all registered pure Rust target drivers.
pub enum ActiveRustDriver {
    Ch32v00x(Ch32v00xDriver),
    Ch32v123xx(Ch32v123xxDriver),
}

impl RustTargetDriver for ActiveRustDriver {
    fn name(&self) -> &'static str {
        match self {
            Self::Ch32v00x(d) => d.name(),
            Self::Ch32v123xx(d) => d.name(),
        }
    }

    fn part_id(&self) -> u32 {
        match self {
            Self::Ch32v00x(d) => d.part_id(),
            Self::Ch32v123xx(d) => d.part_id(),
        }
    }

    fn target_options(&self) -> u32 {
        match self {
            Self::Ch32v00x(d) => d.target_options(),
            Self::Ch32v123xx(d) => d.target_options(),
        }
    }

    fn has_hw_breakpoints(&self) -> bool {
        match self {
            Self::Ch32v00x(d) => d.has_hw_breakpoints(),
            Self::Ch32v123xx(d) => d.has_hw_breakpoints(),
        }
    }

    fn ram_regions(&self) -> &[RamDesc] {
        match self {
            Self::Ch32v00x(d) => d.ram_regions(),
            Self::Ch32v123xx(d) => d.ram_regions(),
        }
    }

    fn flash_regions(&self) -> &[FlashDesc] {
        match self {
            Self::Ch32v00x(d) => d.flash_regions(),
            Self::Ch32v123xx(d) => d.flash_regions(),
        }
    }

    fn flash_prepare(&mut self, hart: &mut rsbmp::riscv::RiscvHart) -> bool {
        match self {
            Self::Ch32v00x(d) => d.flash_prepare(hart),
            Self::Ch32v123xx(d) => d.flash_prepare(hart),
        }
    }

    fn flash_done(&mut self, hart: &mut rsbmp::riscv::RiscvHart) -> bool {
        match self {
            Self::Ch32v00x(d) => d.flash_done(hart),
            Self::Ch32v123xx(d) => d.flash_done(hart),
        }
    }

    fn flash_erase(&mut self, hart: &mut rsbmp::riscv::RiscvHart, addr: u32, len: usize) -> bool {
        match self {
            Self::Ch32v00x(d) => d.flash_erase(hart, addr, len),
            Self::Ch32v123xx(d) => d.flash_erase(hart, addr, len),
        }
    }

    fn flash_write(&mut self, hart: &mut rsbmp::riscv::RiscvHart, dest: u32, data: &[u8]) -> bool {
        match self {
            Self::Ch32v00x(d) => d.flash_write(hart, dest, data),
            Self::Ch32v123xx(d) => d.flash_write(hart, dest, data),
        }
    }

    fn sw_page_size(&self) -> u32 {
        match self {
            Self::Ch32v00x(d) => d.sw_page_size(),
            Self::Ch32v123xx(d) => d.sw_page_size(),
        }
    }

    fn sw_page_erase(&mut self, hart: &mut rsbmp::riscv::RiscvHart, addr: u32) -> bool {
        match self {
            Self::Ch32v00x(d) => d.sw_page_erase(hart, addr),
            Self::Ch32v123xx(d) => d.sw_page_erase(hart, addr),
        }
    }

    fn sw_page_write(&mut self, hart: &mut rsbmp::riscv::RiscvHart, addr: u32, data: &[u8]) -> bool {
        match self {
            Self::Ch32v00x(d) => d.sw_page_write(hart, addr, data),
            Self::Ch32v123xx(d) => d.sw_page_write(hart, addr, data),
        }
    }

    fn crc32(&mut self, hart: &mut rsbmp::riscv::RiscvHart, addr: u32, len: u32) -> Option<u32> {
        match self {
            Self::Ch32v00x(d) => d.crc32(hart, addr, len),
            Self::Ch32v123xx(d) => d.crc32(hart, addr, len),
        }
    }
}

static mut ACTIVE_DRIVER: Option<ActiveRustDriver> = None;

// --- Central TargetFlash C Callbacks ---

unsafe extern "C" fn central_flash_prepare(flash: *mut TargetFlash) -> bool {
    if flash.is_null() {
        return false;
    }
    let target = (*flash).t as *mut TargetC;
    let hart = match get_riscv_hart(target) {
        Some(h) => h,
        None => return false,
    };
    if let Some(ref mut driver) = ACTIVE_DRIVER {
        driver.flash_prepare(hart)
    } else {
        false
    }
}

unsafe extern "C" fn central_flash_erase(flash: *mut TargetFlash, addr: u32, len: usize) -> bool {
    if flash.is_null() {
        return false;
    }
    let target = (*flash).t as *mut TargetC;
    let hart = match get_riscv_hart(target) {
        Some(h) => h,
        None => return false,
    };
    if let Some(ref mut driver) = ACTIVE_DRIVER {
        driver.flash_erase(hart, addr, len)
    } else {
        false
    }
}

unsafe extern "C" fn central_flash_write(
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
    if let Some(ref mut driver) = ACTIVE_DRIVER {
        driver.flash_write(hart, dest, data)
    } else {
        false
    }
}

unsafe extern "C" fn central_flash_done(flash: *mut TargetFlash) -> bool {
    if flash.is_null() {
        return false;
    }
    let target = (*flash).t as *mut TargetC;
    let hart = match get_riscv_hart(target) {
        Some(h) => h,
        None => return false,
    };
    if let Some(ref mut driver) = ACTIVE_DRIVER {
        driver.flash_done(hart)
    } else {
        false
    }
}

// --- Central Software Breakpoint C Callbacks ---

unsafe extern "C" fn central_sw_page_size(_t: *mut TargetC) -> u32 {
    if let Some(ref driver) = ACTIVE_DRIVER {
        driver.sw_page_size()
    } else {
        0
    }
}

unsafe extern "C" fn central_sw_page_erase(t: *mut TargetC, addr: u32) -> bool {
    let hart = match get_riscv_hart(t) {
        Some(h) => h,
        None => return false,
    };
    if let Some(ref mut driver) = ACTIVE_DRIVER {
        driver.sw_page_erase(hart, addr)
    } else {
        false
    }
}

unsafe extern "C" fn central_sw_page_write(
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
    if let Some(ref mut driver) = ACTIVE_DRIVER {
        driver.sw_page_write(hart, addr, data)
    } else {
        false
    }
}

static CENTRAL_SW_HELPERS: SwBreakpointHelpersC = SwBreakpointHelpersC {
    page_size: central_sw_page_size,
    page_erase: central_sw_page_erase,
    page_write: central_sw_page_write,
};

// --- Central CRC32 C Callback ---

/// C `target_s::crc32` hook for Rust-driven targets, reached through
/// `bmp_custom_crc32_c()` exactly like the C drivers' hooks (`gd32_crc32`, `rp_crc32`).
unsafe extern "C" fn central_crc32(t: *mut TargetC, addr: u32, size: usize, crc: *mut u32) -> bool {
    if t.is_null() || crc.is_null() {
        return false;
    }
    let hart = match get_riscv_hart(t) {
        Some(h) => h,
        None => return false,
    };
    if let Some(ref mut driver) = ACTIVE_DRIVER {
        if let Some(value) = driver.crc32(hart, addr, size as u32) {
            *crc = value;
            return true;
        }
    }
    false
}

/// Attach and configure target from a pure Rust driver.
pub unsafe fn central_attach_rust_driver(target: *mut TargetC, driver: ActiveRustDriver) -> bool {
    target_set_driver_c(target, driver.name().as_ptr() as *const c_char);
    target_set_part_id_c(target, driver.part_id());
    target_set_options_c(target, driver.target_options());

    target_add_default_commands_c(target);

    target_mem_map_free(target);
    for ram in driver.ram_regions() {
        target_add_ram32(target, ram.start, ram.length as u32);
    }

    for flash in driver.flash_regions() {
        target_add_flash_region(
            target,
            flash.start,
            flash.length,
            flash.blocksize,
            flash.writesize,
            flash.erasesizes.as_ptr(),
            flash.erased,
            Some(central_flash_prepare),
            Some(central_flash_erase),
            Some(central_flash_write),
            Some(central_flash_done),
        );
    }

    if !driver.has_hw_breakpoints() {
        target_set_sw_breakpoint_helpers_c(target, &CENTRAL_SW_HELPERS);
        target_set_no_hw_breakpoint_c(target, true);
    }

    // Hardware CRC: drivers override `crc32()`; the default returns None and the hook
    // then declines every range, so `qCRC` falls back to the software CRC as before.
    target_set_crc32_c(target, central_crc32);

    ACTIVE_DRIVER = Some(driver);
    true
}

/// Central probe entry point called from `riscv32_silicon_probe_c` in `bmp_helpers.cpp`.
///
/// Probes all registered pure Rust target drivers in priority order.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn riscv_rust_target_probe(target: *mut TargetC) -> bool {
    if target.is_null() {
        return false;
    }

    let hart = match get_riscv_hart(target) {
        Some(h) => h,
        None => return false,
    };

    // 1. Probe CH32V00x / CH32V006 pure Rust driver
    if let Some(driver) = Ch32v00xDriver::probe(hart) {
        return central_attach_rust_driver(target, ActiveRustDriver::Ch32v00x(driver));
    }

    // 2. Probe CH32V1xx / CH32V2xx / CH32V3xx pure Rust driver
    if let Some(driver) = Ch32v123xxDriver::probe(hart) {
        return central_attach_rust_driver(target, ActiveRustDriver::Ch32v123xx(driver));
    }

    false
}
