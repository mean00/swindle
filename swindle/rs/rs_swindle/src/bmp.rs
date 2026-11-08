//! BMP FFI and target control layer — safe Rust wrappers around target drivers and hardware probes.

use crate::SyncCell;
use crate::commands::run::HaltState;
use crate::flash::engine as flash_engine;
use crate::target::CortexMAccess;
use crate::target::driver::TargetDriver;
use crate::target::{self as target_adapter, Target};
use core::ffi::{CStr, c_char, c_uint};

pub const BMP_ARCH_NONE: u32 = 0;
pub const BMP_ARCH_ARM: u32 = 1;
pub const BMP_ARCH_RISCV: u32 = 2;

pub use crate::platform::reset_pin;

#[cfg(not(feature = "hosted"))]
mod c {
    unsafe extern "C" {
        pub fn rvswdp_scan() -> bool;
    }
}

unsafe extern "C" {
    fn bmp_crc32_c(address: u32, length: u32, crc: *mut u32) -> bool;
    fn list_enabled_boards() -> *const c_char;
    fn bmp_get_version_string() -> *const c_char;
    fn free_heap_c() -> u32;
    fn min_free_heap_c() -> u32;
    fn bmp_raise_exception_c();
    fn bmp_enable_reset_pin_c(enabled: bool);
    fn bmp_mem_counts_c(swd: *mut c_uint, rv: *mut c_uint);
    fn bmp_mem_counts_reset_c();
    fn bmp_log_mem_c(is_write: bool, addr: u32, length: u32);
}

static MEM_LOG_ENABLED: SyncCell<bool> = SyncCell::new(false);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum mapping {
    Flash = 0,
    Ram = 1,
}

/// Standardized error type for BMP hardware and protocol operations.
#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum BmpError {
    NotAttached = 1,
    BusFault = 2,
    Timeout = 3,
    ProtocolError = 4,
    Unsupported = 5,
    Exception(i32) = 6,
}

/// Zero-sized token representing an attached target session.
/// Guarantees that target operations are only called within an attached context.
#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub struct AttachedTarget {
    _private: (),
}

#[allow(dead_code)]
impl AttachedTarget {
    /// Obtain an AttachedTarget token if a target is currently attached.
    #[inline(always)]
    pub fn current() -> Option<Self> {
        if bmp_attached() {
            Some(AttachedTarget { _private: () })
        } else {
            None
        }
    }

    /// Read target memory into a byte slice.
    #[inline(always)]
    pub fn read_mem(&self, address: u32, data: &mut [u8]) -> Result<(), BmpError> {
        if bmp_read_mem(address, data) {
            Ok(())
        } else {
            Err(BmpError::BusFault)
        }
    }

    /// Write target memory from a byte slice.
    #[inline(always)]
    pub fn write_mem(&self, address: u32, data: &[u8]) -> Result<(), BmpError> {
        if bmp_mem_write(address, data) {
            Ok(())
        } else {
            Err(BmpError::BusFault)
        }
    }

    /// Read a target register.
    #[inline(always)]
    pub fn read_register(&self, reg: u32) -> Result<u32, BmpError> {
        bmp_read_register(reg).ok_or(BmpError::BusFault)
    }

    /// Write a target register.
    #[inline(always)]
    pub fn write_register(&self, reg: u32, value: u32) -> Result<(), BmpError> {
        if bmp_write_register(reg, value) {
            Ok(())
        } else {
            Err(BmpError::BusFault)
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MemoryBlock {
    pub start_address: u32,
    pub length: u32,
    #[allow(dead_code)]
    pub block_size: u32,
}

/// Get the number of memory blocks in a map (Flash or Ram).
pub fn bmp_map_count(map: mapping) -> usize {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return 0;
    }
    match map {
        mapping::Flash => target_adapter::target_flash_count(cur),
        mapping::Ram => target_adapter::target_ram_count(cur),
    }
}

/// Get a specific memory block from a map by index.
pub fn bmp_map_get(map: mapping, index: usize) -> Option<MemoryBlock> {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return None;
    }
    match map {
        mapping::Flash => {
            target_adapter::target_flash_get(cur, index).map(|(start, len, block_size)| {
                MemoryBlock {
                    start_address: start,
                    length: len,
                    block_size,
                }
            })
        }
        mapping::Ram => {
            target_adapter::target_ram_get(cur, index).map(|(start, len)| MemoryBlock {
                start_address: start,
                length: len,
                block_size: 0,
            })
        }
    }
}

#[allow(dead_code)]
pub fn bmp_register_description() -> &'static str {
    ""
}
#[allow(dead_code)]
pub fn bmp_drop_register_description() {}

/// Largest GDB register file we hand back, in 32-bit words.
pub const MAX_REG_WORDS: usize = 80;
const MAX_REGS_BYTES: usize = MAX_REG_WORDS * 4;

/// Scratch the register file is read into.
static REGS_SCRATCH: SyncCell<[u32; MAX_REG_WORDS]> = SyncCell::new([0; MAX_REG_WORDS]);

/// Read all target registers.
pub fn bmp_read_registers() -> &'static [u32] {
    let Some(driver) = target_adapter::current_driver() else {
        return &[];
    };
    let size = driver.regs_size();
    if size > MAX_REGS_BYTES || size % 4 != 0 {
        crate::bmpwarning!("register file of {} bytes does not fit\n", size);
        return &[];
    }
    let buf = unsafe { core::slice::from_raw_parts_mut(REGS_SCRATCH.as_ptr().cast::<u8>(), size) };
    if driver.regs_read(buf).is_err() {
        return &[];
    }
    let words = unsafe { REGS_SCRATCH.as_ref() };
    &words[..size / 4]
}

/// Largest number of memory-map regions we report per map.
pub const MAX_MAP_BLOCKS: usize = 8;

/// Get the target's memory map (flash or RAM regions).
pub fn bmp_get_mapping(map: mapping) -> heapless::Vec<MemoryBlock, MAX_MAP_BLOCKS> {
    let count = bmp_map_count(map);
    let mut r: heapless::Vec<MemoryBlock, MAX_MAP_BLOCKS> = heapless::Vec::new();
    for i in 0..count {
        if let Some(block) = bmp_map_get(map, i)
            && r.push(block).is_err()
        {
            crate::bmpwarning!("memory map has more than {} blocks\n", MAX_MAP_BLOCKS);
            break;
        }
    }
    r
}

/// Perform an SWD scan to discover attached targets.
#[unsafe(no_mangle)]
pub extern "C" fn swdp_scan() -> bool {
    if let Some(volt_str) = crate::platform::voltage::target_voltage_str() {
        crate::gdb_println!("Target voltage: ", volt_str);
    }
    crate::arm::adi::discovery::swd_scan_targetid(0)
}

/// Perform a RISC-V SWD scan.
pub fn rvswdp_scan() -> bool {
    if let Some(volt_str) = crate::platform::voltage::target_voltage_str() {
        crate::gdb_println!("Target voltage: ", volt_str);
    }
    #[cfg(not(feature = "hosted"))]
    unsafe {
        c::rvswdp_scan()
    }
    #[cfg(feature = "hosted")]
    {
        crate::hosted::dmi_scan::rvswdp_scan()
    }
}

/// Comprehensive cleanup executed on target disconnect/detach.
pub fn disconnect_cleanup(target: *mut Target) {
    let t = if target.is_null() {
        target_adapter::get_cur_target()
    } else {
        target
    };

    crate::mem_cache::invalidate();
    crate::freertos::os_detach();
    crate::sw_breakpoints::clear_all_breakpoints();
    crate::commands::mon::clear_custom_target_command();
    crate::rtt::swindle_enable_rtt(false);

    crate::target::breakwatch::clear_all_on(t);

    if !t.is_null() {
        crate::flash::flash_cleanup(t as _);
    } else {
        crate::flash::flash_cleanup(core::ptr::null_mut());
    }
}

/// Detach from the currently attached target.
pub fn bmp_detach() -> bool {
    let cur = target_adapter::get_cur_target();
    disconnect_cleanup(cur);
    if !cur.is_null() {
        bmp_gpio_reset();
        unsafe { target_adapter::target_detach(cur) };
        target_adapter::set_cur_target(core::ptr::null_mut());
    }
    true
}

/// Read the target's supply voltage.
pub fn bmp_get_target_voltage() -> f32 {
    crate::platform::voltage::target_voltage_volts()
}

/// Check if a target is currently attached.
pub fn bmp_attached() -> bool {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        false
    } else {
        target_adapter::TargetView::new(cur).attached()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_attached_c() -> bool {
    bmp_attached()
}

/// Attach to a target by index.
pub fn bmp_attach(target: u32) -> bool {
    crate::mem_cache::invalidate();
    let t = unsafe { target_adapter::target_attach_n(target as usize) };
    target_adapter::set_cur_target(t);
    !t.is_null()
}

/// Write a single target register.
pub fn bmp_write_register(reg: u32, value: u32) -> bool {
    if let Some(driver) = target_adapter::current_driver() {
        let bytes = value.to_le_bytes();
        driver.reg_write(reg, &bytes).is_ok()
    } else {
        false
    }
}

/// Read a single target register.
pub fn bmp_read_register(reg: u32) -> Option<u32> {
    if let Some(driver) = target_adapter::current_driver() {
        let mut buf = [0u8; 4];
        if driver.reg_read(reg, &mut buf).is_ok() {
            Some(u32::from_le_bytes(buf))
        } else {
            None
        }
    } else {
        None
    }
}

/// Erase a region of target flash memory.
pub fn bmp_flash_erase(adr: u32, size: u32) -> bool {
    crate::mem_cache::invalidate();
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return false;
    }
    flash_engine::target_flash_erase(cur as *mut _, adr, size as usize)
}

/// Write data to target flash memory.
pub fn bmp_flash_write(addr: u32, data: &[u8]) -> bool {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return false;
    }
    flash_engine::target_flash_write(cur as *mut _, addr, data.as_ptr(), data.len())
}

/// Get the target's driver name.
pub fn bmp_get_target_name() -> &'static str {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        ""
    } else {
        target_adapter::TargetView::new(cur)
            .driver_str()
            .unwrap_or("")
    }
}

/// Write data to target memory (RAM).
pub fn bmp_mem_write(address: u32, data: &[u8]) -> bool {
    crate::mem_cache::invalidate();
    let Some(driver) = target_adapter::current_driver() else {
        return false;
    };
    if !MEM_LOG_ENABLED.get() {
        return driver.mem_write(address, data).is_ok();
    }
    unsafe { bmp_log_mem_c(true, 0, 0) };
    let ok = driver.mem_write(address, data).is_ok();
    unsafe { bmp_log_mem_c(true, address, data.len() as u32) };
    ok
}

/// Write 32-bit words to target memory.
pub fn bmp_write_mem32(address: u32, data: &[u32]) -> bool {
    crate::mem_cache::invalidate();
    let bytes = unsafe { core::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) };
    bmp_mem_write(address, bytes)
}

/// Read the target CPUID register.
pub fn bmp_cpuid() -> u32 {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        0
    } else {
        target_adapter::TargetView::new(cur).cpuid()
    }
}

/// Finalise a flash programming operation.
pub fn bmp_flash_complete() -> bool {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return false;
    }
    flash_engine::target_flash_complete(cur as *mut _)
}

/// Compute CRC32 of target memory (BMP internal implementation).
pub fn _bmp_crc32(address: u32, length: u32) -> Option<u32> {
    unsafe {
        let mut crc: u32 = 0;
        if bmp_crc32_c(address, length, &mut crc) {
            return Some(crc);
        }
        None
    }
}

/// Read target memory into a raw pointer.
pub fn bmp_read_mem_ptr(address: u32, size: u32, data: *mut u8) -> bool {
    if data.is_null() {
        return false;
    }
    let slice = unsafe { core::slice::from_raw_parts_mut(data, size as usize) };
    bmp_read_mem(address, slice)
}

/// Ask the target whether memory access requires halting the CPU first.
pub fn bmp_mem_access_needs_halt() -> bool {
    if let Some(driver) = target_adapter::current_driver() {
        driver.needs_halt_for_mem()
    } else {
        true
    }
}

/// Read target memory without halting the CPU.
pub fn bmp_mem_read_nostop(address: u32, size: u32, data: *mut u8) -> bool {
    if data.is_null() {
        return false;
    }
    if let Some(driver) = target_adapter::current_driver() {
        let slice = unsafe { core::slice::from_raw_parts_mut(data, size as usize) };
        driver.mem_read_nostop(slice, address).unwrap_or(false)
    } else {
        false
    }
}

/// Write target memory without halting the CPU.
pub fn bmp_mem_write_nostop(address: u32, size: u32, data: *const u8) -> bool {
    if data.is_null() {
        return false;
    }
    if let Some(driver) = target_adapter::current_driver() {
        let slice = unsafe { core::slice::from_raw_parts(data, size as usize) };
        driver.mem_write_nostop(address, slice).unwrap_or(false)
    } else {
        false
    }
}

/// Read target memory into a byte slice.
pub fn bmp_read_mem(address: u32, data: &mut [u8]) -> bool {
    let Some(driver) = target_adapter::current_driver() else {
        return false;
    };
    if !MEM_LOG_ENABLED.get() {
        return driver.mem_read(data, address).is_ok();
    }
    unsafe { bmp_log_mem_c(false, 0, 0) };
    let ok = driver.mem_read(data, address).is_ok();
    unsafe { bmp_log_mem_c(false, address, data.len() as u32) };
    ok
}

/// Read 32-bit words from target memory.
pub fn bmp_read_mem32(address: u32, data: &mut [u32]) -> bool {
    let bytes =
        unsafe { core::slice::from_raw_parts_mut(data.as_mut_ptr() as *mut u8, data.len() * 4) };
    bmp_read_mem(address, bytes)
}

/// Enable/disable the per-call memory-access instrumentation log (Phase 0).
pub fn bmp_set_mem_log(enable: bool) {
    MEM_LOG_ENABLED.set(enable);
}

/// Snapshot the global SWD/RVSWD wire-transaction counters.
pub fn bmp_mem_counts() -> (u32, u32) {
    unsafe {
        let mut swd: u32 = 0;
        let mut rv: u32 = 0;
        bmp_mem_counts_c(&mut swd, &mut rv);
        (swd, rv)
    }
}

/// Zero the global SWD/RVSWD wire-transaction counters (Phase 0).
pub fn bmp_mem_counts_reset() {
    unsafe { bmp_mem_counts_reset_c() }
}

/// Reset the target MCU.
pub fn bmp_reset_target() -> bool {
    if let Some(driver) = target_adapter::current_driver() {
        driver.reset();
        true
    } else {
        false
    }
}

/// Set SWD wait states (slows down the SWD clock).
pub fn bmp_set_wait_state(ws: u32) {
    crate::platform::timing::set_wait_states(ws);
}

/// Get the current SWD wait state value.
pub fn bmp_get_wait_state() -> u32 {
    crate::platform::timing::wait_states()
}

/// Set the SWD clock frequency.
pub fn bmp_set_frequency(fq: u32) {
    crate::platform::timing::set_frequency(fq);
}

/// Get the current SWD clock frequency in Hz.
pub fn bmp_get_frequency() -> u32 {
    crate::platform::timing::frequency()
}

/// Halt the target MCU execution.
pub fn bmp_target_halt() -> bool {
    if let Some(driver) = target_adapter::current_driver() {
        driver.halt_request();
        true
    } else {
        false
    }
}

/// Resume target execution, optionally stepping a single instruction.
pub fn bmp_halt_resume(step: bool) -> bool {
    if let Some(driver) = target_adapter::current_driver() {
        driver.halt_resume(step);
        true
    } else {
        false
    }
}

/// Poll the target's current state.
pub fn bmp_poll() -> HaltState {
    if let Some(driver) = target_adapter::current_driver() {
        let (reason, wp) = driver.halt_poll();
        match reason {
            crate::target::driver::HaltReason::Running => HaltState::Running,
            crate::target::driver::HaltReason::Error => HaltState::Error,
            crate::target::driver::HaltReason::Request => HaltState::Request,
            crate::target::driver::HaltReason::Stepping => HaltState::Stepping,
            crate::target::driver::HaltReason::Breakpoint => HaltState::Breakpoint,
            crate::target::driver::HaltReason::Watchpoint => HaltState::Watchpoint(wp),
            crate::target::driver::HaltReason::Fault => HaltState::Fault,
            crate::target::driver::HaltReason::Unknown => HaltState::Error,
        }
    } else {
        HaltState::Error
    }
}

// Native-only BMP functions are in native::bmp_native, re-exported below.
#[cfg(not(feature = "hosted"))]
pub use crate::native::bmp_native::*;

/// Get a list of supported target boards.
pub fn bmp_supported_boards() -> &'static str {
    unsafe {
        let boards = list_enabled_boards();
        core::str::from_utf8_unchecked(CStr::from_ptr(boards).to_bytes())
    }
}

/// Get the BMP firmware version string.
pub fn bmp_get_version() -> &'static str {
    unsafe { core::str::from_utf8_unchecked(CStr::from_ptr(bmp_get_version_string()).to_bytes()) }
}

/// Execute a BMP monitor command.
pub fn bmp_mon(_input_as_string: &str) -> bool {
    false
}

/// Get the current free heap size in bytes.
pub fn free_heap() -> u32 {
    unsafe { free_heap_c() }
}

/// Get the minimum free heap size since boot (watermark).
pub fn min_free_heap() -> u32 {
    unsafe { min_free_heap_c() }
}

/// Get heap statistics as (min_free, current_free).
pub fn get_heap_stats() -> (u32, u32) {
    (min_free_heap(), free_heap())
}

/// Check if the attached target is a RISC-V core.
pub fn bmp_is_riscv() -> bool {
    bmp_get_arch() == bmp_arch::BMP_ARCH_RISCV
}

/// Write a CH32V3xx user option byte.
pub fn bmp_ch32v3xx_write_user_option_byte(_memory_conf: u8) -> bool {
    false
}

/// Compute CRC32 using the target's hardware CRC unit.
pub fn bmp_custom_crc32(address: u32, size_in_bytes: u32) -> (bool, u32) {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return (false, 0);
    }
    if let (Some(driver), Some(hart)) = (
        crate::riscv::driver_registry::active_driver(),
        crate::riscv::current_riscv_hart(),
    ) {
        return match driver.crc32(hart, address, size_in_bytes) {
            Some(crc) => (true, crc),
            None => (false, 0),
        };
    }
    if let Some(driver) = crate::arm::driver_registry::active_driver_for(cur) {
        let mut target = CortexMAccess::new(cur);
        return match driver.crc32(&mut target, address, size_in_bytes) {
            Some(crc) => (true, crc),
            None => (false, 0),
        };
    }
    (false, 0)
}

/// Raise a simulated exception for GDB.
pub fn bmp_raise_exception() {
    unsafe {
        bmp_raise_exception_c();
    }
}

#[allow(dead_code)]
pub fn bmp_try<F: FnMut()>(mut f: F) -> Result<(), i32> {
    f();
    Ok(())
}

/// Execute a closure within a BMP exception handling context and return `Result<(), BmpError>`.
#[allow(dead_code)]
pub fn bmp_try_res<F: FnMut()>(mut f: F) -> Result<(), BmpError> {
    f();
    Ok(())
}

/// Enable or disable the target reset pin control.
pub fn _bmp_enable_reset_pin(enabled: bool) {
    unsafe {
        bmp_enable_reset_pin_c(enabled);
    }
}

/// Get the number of available watchpoint and breakpoint slots.
pub fn bmp_watchpoint_breakpoint_count() -> (u32, u32) {
    (0, 0)
}

/// Check if the target supports hardware breakpoints.
pub fn bmp_has_hw_breakpoint() -> bool {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        false
    } else {
        target_adapter::TargetView::new(cur).has_hw_breakpoint()
    }
}

/// Check if the target has mass write helpers (for faster flash programming).
pub fn bmp_has_mw_helpers() -> bool {
    crate::riscv::driver_registry::active_driver().is_some_and(|d| d.sw_page_size() != 0)
}

/// Overwrite flash memory (erase + write in one operation).
pub fn bmp_overwrite_flash(address: u32, data: &[u8]) -> bool {
    let (Some(driver), Some(hart)) = (
        crate::riscv::driver_registry::active_driver(),
        crate::riscv::current_riscv_hart(),
    ) else {
        return false;
    };
    if driver.sw_page_size() != data.len() as u32 {
        return false;
    }
    if !driver.sw_page_erase(hart, address) {
        return false;
    }
    driver.sw_page_write(hart, address, data)
}

/// Get the mass write page size in bytes.
pub fn bmp_get_mw_page_size() -> u32 {
    crate::riscv::driver_registry::active_driver().map_or(0, |d| d.sw_page_size())
}

/// Is `address` inside one of the target's flash regions?
pub fn bmp_is_flash_addr(address: u32) -> bool {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        false
    } else {
        target_adapter::target_addr_is_flash(cur, address)
    }
}

/// Target architecture enum.
#[allow(dead_code)]
#[derive(PartialEq)]
#[repr(u32)]
pub enum bmp_arch {
    BMP_ARCH_NONE = BMP_ARCH_NONE,
    BMP_ARCH_ARM = BMP_ARCH_ARM,
    BMP_ARCH_RISCV = BMP_ARCH_RISCV,
}

/// Reset SWDIO GPIO state (set high + output).
#[allow(dead_code)]
pub fn bmp_gpio_reset() {
    crate::platform::swd::gpio_reset();
}

/// Get the target's architecture (ARM or RISC-V).
#[allow(dead_code)]
pub fn bmp_get_arch() -> bmp_arch {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return bmp_arch::BMP_ARCH_NONE;
    }
    let opts = target_adapter::TargetView::new(cur).options();
    match (opts >> 28) & 0x03 {
        1 => bmp_arch::BMP_ARCH_ARM,
        2 => bmp_arch::BMP_ARCH_RISCV,
        _ => bmp_arch::BMP_ARCH_NONE,
    }
}

/// Get the target's raw target_options bitmask.
pub fn bmp_get_target_options() -> u32 {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        0
    } else {
        target_adapter::TargetView::new(cur).options()
    }
}

/// Get RISC-V hart info (address width in bits, ISA extension flags).
pub fn bmp_get_riscv_info() -> Option<(u8, u32)> {
    let ctx = crate::riscv::current_riscv_context()?;
    let exts = match ctx.hart.isa {
        crate::riscv::HartIsa::Rv32e => {
            crate::target_xml::RV_ISA_EXT_EMBEDDED | crate::target_xml::RV_ISA_EXT_COMPRESSED
        }
        crate::riscv::HartIsa::Rv32i => {
            crate::target_xml::RV_ISA_EXT_INTEGER | crate::target_xml::RV_ISA_EXT_COMPRESSED
        }
        crate::riscv::HartIsa::Rv32iSingleFpu => {
            crate::target_xml::RV_ISA_EXT_INTEGER
                | crate::target_xml::RV_ISA_EXT_COMPRESSED
                | crate::target_xml::RV_ISA_EXT_SINGLE_FLOAT
        }
    };
    Some((32, exts))
}

/// The number of counters `bmp_riscv_confirm_stats()` fills (riscv32.c, `riscv_confirm_stats_s`).
pub const RISCV_CONFIRM_STATS_FIELDS: usize = 9;

/// Select the RISC-V program-buffer streaming sub-path ('mon riscv_stream').
pub fn bmp_set_riscv_progbuf_stream(_enable: bool) {}

/// Is the RISC-V program-buffer streaming sub-path selected?
pub fn bmp_get_riscv_progbuf_stream() -> bool {
    false
}

/// Snapshot the RISC-V read-confirm counters, in the order riscv32.c declares them.
pub fn bmp_riscv_confirm_stats() -> [u32; RISCV_CONFIRM_STATS_FIELDS] {
    [0u32; RISCV_CONFIRM_STATS_FIELDS]
}

/// Zero the RISC-V read-confirm counters.
pub fn bmp_riscv_confirm_stats_reset() {}

/// Run the RISC-V memory benchmark directly.
pub fn bmp_run_riscv_benchmark() -> bool {
    let Some(ctx) = crate::riscv::current_riscv_context() else {
        crate::gdb_println!("Error: No RISC-V target attached.");
        return false;
    };
    crate::riscv::benchmark::run_benchmark(&mut ctx.hart)
}

/// Run the RISC-V memory benchmark through the generic target memory API.
pub fn bmp_run_riscv_benchmark2() -> bool {
    let Some(ctx) = crate::riscv::current_riscv_context() else {
        crate::gdb_println!("Error: No RISC-V target attached.");
        return false;
    };
    crate::riscv::benchmark::run_benchmark2(&mut ctx.hart)
}

/// Read a RISC-V CSR.
pub fn bmp_riscv_read_csr(csr: u32) -> Option<u32> {
    let ctx = crate::riscv::current_riscv_context()?;
    ctx.hart.csr_read(csr as u16).ok()
}

/// Write a RISC-V CSR.
pub fn bmp_riscv_write_csr(csr: u32, val: u32) -> bool {
    let Some(ctx) = crate::riscv::current_riscv_context() else {
        return false;
    };
    ctx.hart.csr_write(csr as u16, val).is_ok()
}
// EOF
