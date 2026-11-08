//! Flash backend dispatch and execution engine for target flash programming.

#![allow(unsafe_op_in_unsafe_fn)]

use super::driver::{FlashDriver, FlashError, FlashOperation};
use super::geometry::FlashGeometry;
use super::manager::{FlashManager, FlashRegion};
use crate::sync_cell::SyncCell;
use crate::target::{CortexMAccess, FlashDesc, Target, TargetDriver};

fn target_enter_flash_mode(target: *mut Target) -> bool {
    if target.is_null() {
        return false;
    }
    unsafe {
        if (*target).flash_mode {
            return true;
        }
        if !(*target).enter_flash_mode_stub
            && let Some(d) = crate::target::driver_for(target)
        {
            d.reset();
        }
        (*target).flash_mode = true;
        true
    }
}

fn target_exit_flash_mode(target: *mut Target) -> bool {
    if target.is_null() {
        return false;
    }
    unsafe {
        if !(*target).flash_mode {
            return true;
        }
        if !(*target).enter_flash_mode_stub
            && let Some(d) = crate::target::driver_for(target)
        {
            d.reset();
        }
        (*target).flash_mode = false;
        true
    }
}

fn target_get_flash_mode(target: *mut Target) -> bool {
    if target.is_null() {
        false
    } else {
        unsafe { (*target).flash_mode }
    }
}

/// Unified flash driver backend: dispatches directly to Rust ARM/RISC-V drivers.
pub enum FlashBackend {
    ArmRust(*mut Target),
    RiscvRust(*mut Target),
}

impl FlashDriver for FlashBackend {
    fn prepare(&mut self, op: FlashOperation) -> Result<(), FlashError> {
        match *self {
            Self::ArmRust(t) => {
                let driver = crate::arm::driver_registry::active_driver_for(t)
                    .ok_or(FlashError::DriverFailed)?;
                if driver.flash_prepare(&mut CortexMAccess::new(t), op) {
                    Ok(())
                } else {
                    Err(FlashError::DriverFailed)
                }
            }
            Self::RiscvRust(t) => {
                let hart = crate::riscv::get_riscv_hart(t).ok_or(FlashError::DriverFailed)?;
                let driver = crate::riscv::driver_registry::active_driver()
                    .ok_or(FlashError::DriverFailed)?;
                if driver.flash_prepare(hart, op) {
                    Ok(())
                } else {
                    Err(FlashError::DriverFailed)
                }
            }
        }
    }

    fn erase_block(&mut self, addr: u32, len: usize) -> Result<(), FlashError> {
        match *self {
            Self::ArmRust(t) => {
                let driver = crate::arm::driver_registry::active_driver_for(t)
                    .ok_or(FlashError::DriverFailed)?;
                if driver.flash_erase(&mut CortexMAccess::new(t), addr, len) {
                    Ok(())
                } else {
                    Err(FlashError::DriverFailed)
                }
            }
            Self::RiscvRust(t) => {
                let hart = crate::riscv::get_riscv_hart(t).ok_or(FlashError::DriverFailed)?;
                let driver = crate::riscv::driver_registry::active_driver()
                    .ok_or(FlashError::DriverFailed)?;
                if driver.flash_erase(hart, addr, len) {
                    Ok(())
                } else {
                    Err(FlashError::DriverFailed)
                }
            }
        }
    }

    fn has_mass_erase(&self) -> bool {
        false
    }

    fn mass_erase(&mut self) -> Result<(), FlashError> {
        Err(FlashError::DriverFailed)
    }

    fn write_block(&mut self, dest: u32, data: &[u8]) -> Result<(), FlashError> {
        // Kept for flash-path debugging (e.g. a payload that never reaches the part):
        // bmpwarning!(
        //     "FLASHSTAT write_block len={} dest=0x{:x} head={:02x}{:02x}{:02x}{:02x} {:02x}{:02x}{:02x}{:02x}\n",
        //     data.len() as u32,
        //     dest,
        //     data[0],
        //     data[1],
        //     data[2],
        //     data[3],
        //     data[4],
        //     data[5],
        //     data[6],
        //     data[7]
        // );
        match *self {
            Self::ArmRust(t) => {
                let driver = crate::arm::driver_registry::active_driver_for(t)
                    .ok_or(FlashError::DriverFailed)?;
                if driver.flash_write(&mut CortexMAccess::new(t), dest, data) {
                    Ok(())
                } else {
                    Err(FlashError::DriverFailed)
                }
            }
            Self::RiscvRust(t) => {
                let hart = crate::riscv::get_riscv_hart(t).ok_or(FlashError::DriverFailed)?;
                let driver = crate::riscv::driver_registry::active_driver()
                    .ok_or(FlashError::DriverFailed)?;
                if driver.flash_write(hart, dest, data) {
                    Ok(())
                } else {
                    Err(FlashError::DriverFailed)
                }
            }
        }
    }

    fn done(&mut self) -> Result<(), FlashError> {
        match *self {
            Self::ArmRust(t) => {
                let driver = crate::arm::driver_registry::active_driver_for(t)
                    .ok_or(FlashError::DriverFailed)?;
                if driver.flash_done(&mut CortexMAccess::new(t)) {
                    Ok(())
                } else {
                    Err(FlashError::DriverFailed)
                }
            }
            Self::RiscvRust(t) => {
                let hart = crate::riscv::get_riscv_hart(t).ok_or(FlashError::DriverFailed)?;
                let driver = crate::riscv::driver_registry::active_driver()
                    .ok_or(FlashError::DriverFailed)?;
                if driver.flash_done(hart) {
                    Ok(())
                } else {
                    Err(FlashError::DriverFailed)
                }
            }
        }
    }
}

/// Static state for active flash programming session.
/// Kept as a direct `FlashManager` (`new()` is all-zeros) rather than `Option<FlashManager>`
/// so the 2.3 KB buffer lives in `.bss` instead of `.data` (and avoids a 2.3 KB stack copy
/// when constructing a new session). `ACTIVE_TARGET` being non-null marks an active session.
static ACTIVE_MANAGER: SyncCell<FlashManager<FlashBackend>> = SyncCell::new(FlashManager::new());
static ACTIVE_TARGET: SyncCell<*mut Target> = SyncCell::new(core::ptr::null_mut());

/// Convert a Rust [`FlashDesc`] into a [`FlashRegion`] using the same `writebufsize`
/// sizing rule as C `target_flash_setup_c`.
fn region_from_desc(desc: &FlashDesc, backend: FlashBackend) -> FlashRegion<FlashBackend> {
    let writesize = if desc.writesize == 0 {
        desc.blocksize
    } else {
        desc.writesize
    };
    let writebufsize = if writesize > 0 && writesize <= 1024 / 2 {
        writesize * (1024 / writesize)
    } else {
        writesize
    };
    let geom = FlashGeometry::new(
        desc.start,
        desc.length,
        desc.blocksize,
        writesize,
        writebufsize,
        desc.erased,
    );
    let mut extra_sizes = heapless::Vec::new();
    for &sz in desc.erasesizes {
        if sz == 0 {
            break;
        }
        let _ = extra_sizes.push(sz);
    }
    FlashRegion::new(geom, extra_sizes, backend)
}

fn get_or_create_manager(target: *mut Target) -> Option<&'static mut FlashManager<FlashBackend>> {
    if target.is_null() {
        return None;
    }
    let mgr = unsafe { ACTIVE_MANAGER.as_mut() };
    let target_slot = unsafe { ACTIVE_TARGET.as_mut() };
    if !(*target_slot).is_null() {
        if *target_slot == target {
            return Some(mgr);
        }
        // Target changed: finalize previous manager
        let _ = mgr.complete();
        *target_slot = core::ptr::null_mut();
    }

    mgr.regions.clear();
    mgr.active_idx = None;
    mgr.buffer.clear();
    let target_c = target;

    // The architecture of the target decides which registry may answer, in the same order as
    // `target::driver_for()`. The two families are not interchangeable: their `part_id`s come
    // from unrelated id spaces (an ARM DBGMCU device id vs a RISC-V chip id), so a
    // cross-architecture fallback could only ever pick a wrong driver.
    if crate::riscv::get_riscv_context(target_c).is_some() {
        let Some(driver) = crate::riscv::driver_registry::active_driver() else {
            bmpwarning!("flash: no active RISC-V driver for target\n");
            return None;
        };
        let map = driver.memory_map();
        for desc in map.flash_regions() {
            mgr.add_region(region_from_desc(desc, FlashBackend::RiscvRust(target_c)));
        }
    } else if crate::arm::cortexm::is_cortexm(target) {
        let Some(driver) = crate::arm::driver_registry::active_driver_for(target_c) else {
            bmpwarning!("flash: no active ARM driver for part 0x{:x}\n", unsafe {
                (*target).part_id as u32
            });
            return None;
        };
        let map = driver.memory_map();
        for desc in map.flash_regions() {
            mgr.add_region(region_from_desc(desc, FlashBackend::ArmRust(target_c)));
        }
    } else {
        bmpwarning!("flash: target is neither RISC-V nor Cortex-M\n");
        return None;
    }

    *target_slot = target;
    Some(mgr)
}

// ---------------------------------------------------------------------------
// Flash Operations
// ---------------------------------------------------------------------------

pub fn target_flash_erase(target: *mut Target, addr: u32, len: usize) -> bool {
    if !target_enter_flash_mode(target) {
        bmpwarning!("flash_erase: enter_flash_mode failed\n");
        return false;
    }

    let mgr = match get_or_create_manager(target) {
        Some(m) => m,
        None => {
            bmpwarning!("flash_erase: get_or_create_manager failed\n");
            return false;
        }
    };

    match mgr.erase(addr, len) {
        Ok(_) => true,
        Err(_e) => {
            bmpwarning!(
                "flash_erase: mgr.erase failed at 0x{:x} len 0x{:x}\n",
                addr,
                len
            );
            false
        }
    }
}

pub fn target_flash_write(target: *mut Target, dest: u32, src: *const u8, len: usize) -> bool {
    if !target_enter_flash_mode(target) {
        bmpwarning!("flash_write: enter_flash_mode failed\n");
        return false;
    }

    if src.is_null() && len > 0 {
        bmpwarning!("flash_write: src is null\n");
        return false;
    }
    let slice = unsafe { core::slice::from_raw_parts(src, len) };
    // Kept for flash-path debugging (e.g. a payload that never reaches the part):
    // let mut head = [0u8; 8];
    // let head_len = core::cmp::min(8, len);
    // head[..head_len].copy_from_slice(&slice[..head_len]);
    // bmpwarning!(
    //     "FLASHSTAT target_flash_write dest=0x{:x} len={} head={:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}\n",
    //     dest,
    //     len,
    //     head[0],
    //     head[1],
    //     head[2],
    //     head[3],
    //     head[4],
    //     head[5],
    //     head[6],
    //     head[7]
    // );
    let mgr = match get_or_create_manager(target) {
        Some(m) => m,
        None => {
            bmpwarning!("flash_write: get_or_create_manager failed\n");
            return false;
        }
    };

    match mgr.write(dest, slice) {
        Ok(_) => true,
        Err(_e) => {
            bmpwarning!("flash_write: mgr.write failed at 0x{:x}\n", dest);
            false
        }
    }
}

pub fn target_flash_complete(target: *mut Target) -> bool {
    if target.is_null() || !target_get_flash_mode(target) {
        return false;
    }

    let mut result = true;
    let target_slot = unsafe { ACTIVE_TARGET.as_mut() };
    if !(*target_slot).is_null() {
        let mgr = unsafe { ACTIVE_MANAGER.as_mut() };
        result = mgr.complete().is_ok();
        mgr.regions.clear();
        mgr.active_idx = None;
        mgr.buffer.clear();
        *target_slot = core::ptr::null_mut();
    }

    // Kept for flash-path debugging (stub run statistics of the last session):
    // bmpwarning!(
    // "FLASHSTAT stub_runs={} halted={} wait_ms={} hold_ms={} nostop_pc={} nostop_sp={} code={},{}
    // ",
    // crate::riscv::stub::STUB_STAT_RUNS.load(core::sync::atomic::Ordering::Relaxed),
    // crate::riscv::stub::STUB_STAT_HALTED.load(core::sync::atomic::Ordering::Relaxed),
    // crate::riscv::stub::STUB_STAT_WAIT_MS.load(core::sync::atomic::Ordering::Relaxed),
    // crate::riscv::stub::STUB_STAT_HOLD_MS.load(core::sync::atomic::Ordering::Relaxed),
    // crate::riscv::stub::STUB_STAT_PC.load(core::sync::atomic::Ordering::Relaxed),
    // crate::riscv::stub::STUB_STAT_SP.load(core::sync::atomic::Ordering::Relaxed),
    // crate::riscv::stub::STUB_STAT_CODE0.load(core::sync::atomic::Ordering::Relaxed),
    // crate::riscv::stub::STUB_STAT_CODE1.load(core::sync::atomic::Ordering::Relaxed)
    // );

    target_exit_flash_mode(target);
    result
}

/// Cleanup any active flash programming session and exit flash mode if needed.
pub fn flash_cleanup(target: *mut Target) {
    unsafe {
        if !target.is_null() && target_get_flash_mode(target) {
            target_exit_flash_mode(target);
        }
        let mgr = ACTIVE_MANAGER.as_mut();
        mgr.regions.clear();
        mgr.active_idx = None;
        mgr.buffer.clear();
        *ACTIVE_TARGET.as_mut() = core::ptr::null_mut();
    }
}
