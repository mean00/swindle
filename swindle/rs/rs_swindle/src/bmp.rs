//! BMP C FFI bindings — safe Rust wrappers around the Black Magic Probe C library.
//!
//! Each function wraps a corresponding `rn_bmp_cmd_c` C call, converting between
//! Rust-safe types and C pointers. The BMP C library owns the actual debug hardware
//! access (SWD bit-banging, target memory read/write, flash programming).

use crate::commands::run::HaltState;
use crate::flash::c_adapter as flash_adapter;
use crate::rn_bmp_cmd_c;
use crate::target::c_adapter as target_adapter;
use crate::target::driver::TargetDriver;
use alloc::vec::Vec;
use core::ffi::{CStr, c_char};
use core::ptr::null;
use core::ptr::null_mut;

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
        mapping::Flash => target_adapter::target_flash_get(cur, index).map(|(start, len, block_size)| MemoryBlock {
            start_address: start,
            length: len,
            block_size,
        }),
        mapping::Ram => target_adapter::target_ram_get(cur, index).map(|(start, len)| MemoryBlock {
            start_address: start,
            length: len,
            block_size: 0,
        }),
    }
}

/// Get the XML target register description from BMP.
///
/// Returns a string containing the GDB XML register description for the
/// currently attached target. This is used by GDB to understand the register
/// set (names, types, groups).
#[allow(dead_code)]
pub fn bmp_register_description() -> &'static str {
    ""
}
/// Free the register description string allocated by BMP.
#[allow(dead_code)]
pub fn bmp_drop_register_description() {}

/// Read all target registers into a vector.
///
/// Returns a `Vec<u32>` containing all registers in the order defined by the
/// target's GDB register description. Returns an empty vector if no target
/// is attached.
pub fn bmp_read_registers() -> Vec<u32> {
    if let Some(driver) = target_adapter::current_driver() {
        let size = driver.regs_size();
        let mut buf = vec![0u8; size];
        if driver.regs_read(&mut buf).is_err() {
            return Vec::new();
        }
        let count = size / 4;
        let mut r = Vec::with_capacity(count);
        for chunk in buf.chunks_exact(4) {
            r.push(u32::from_le_bytes(chunk.try_into().unwrap()));
        }
        r
    } else {
        Vec::new()
    }
}
/// Get the target's memory map (flash or RAM regions).
///
/// Queries the BMP target for its flash or RAM memory map and returns a
/// vector of `MemoryBlock` descriptors. Each block describes a contiguous
/// region with its start address, length, and (for flash) block size.
pub fn bmp_get_mapping(map: mapping) -> Vec<MemoryBlock> {
    let count = bmp_map_count(map);
    let mut r: Vec<MemoryBlock> = Vec::with_capacity(count);
    for i in 0..count {
        if let Some(block) = bmp_map_get(map, i) {
            r.push(block);
        }
    }
    r
}
/// Perform an SWD scan to discover attached targets.
///
/// Initiates an SWD bus scan. Returns `true` if at least one target was found.
pub fn swdp_scan() -> bool {
    unsafe { rn_bmp_cmd_c::cmd_swd_scan(null(), 0, null_mut()) }
}
/// Perform a RISC-V SWD scan.
///
/// Initiates an SWD scan specifically for RISC-V targets. Returns `true` if
/// a RISC-V target was found.
pub fn rvswdp_scan() -> bool {
    unsafe { rn_bmp_cmd_c::cmd_rvswd_scan(null(), 0, null_mut()) }
}
// NOTE: the SDI side of this file (sdi_scan(), and the 'mon sdi_wire' knobs
// further down) moved to the optional SDI module — crate::riscv_extra, i.e.
// riscv_extra_sdi.rs behind the `sdi` cargo feature, or
// riscv_extra_sdi_stubs.rs without it. The C half of the same seam is
/// Comprehensive cleanup executed on target disconnect/detach:
/// - Invalidate GDB read-ahead memory line cache
/// - Teardown FreeRTOS OS thread awareness, symbols, and cached TCBs
/// - Clear all RAM and Flash software breakpoints
/// - Clear any target-specific custom monitor commands
/// - Disable RTT polling
/// - Clean up any active flash programming session and exit flash mode
/// - Free any allocated breakwatch nodes on the target
pub fn disconnect_cleanup(target: *mut target_adapter::TargetC) {
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

    if !t.is_null() {
        crate::flash::flash_cleanup(t as _);
        unsafe { target_adapter::free_breakwatch(t) };
    } else {
        crate::flash::flash_cleanup(core::ptr::null_mut());
    }
}

/// Detach from the currently attached target.
///
/// Releases the target, disables RTT, and resets the GPIO state. Returns
/// `true` on success.
pub fn bmp_detach() -> bool {
    let cur = target_adapter::get_cur_target();
    disconnect_cleanup(cur);
    unsafe { rn_bmp_cmd_c::bmp_detach_c(0) }
}

/// Read the target's supply voltage.
///
/// Returns the voltage measured on the target's VREF pin, in volts.
pub fn bmp_get_target_voltage() -> f32 {
    unsafe { rn_bmp_cmd_c::bmp_get_target_voltage_c() }
}

/// Check if a target is currently attached.
pub fn bmp_attached() -> bool {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        false
    } else {
        unsafe { target_adapter::target_get_attached_c(cur) }
    }
}
/// Attach to a target by index.
///
/// `target` is the zero-based index from the SWD scan results. Returns `true`
/// if attachment succeeded.
pub fn bmp_attach(target: u32) -> bool {
    // A different target may now own this address space: a cached line filled
    // while a previous target was attached must never be served on the next
    // attach (the line's RAM/flash map region is target-specific).
    crate::mem_cache::invalidate();
    unsafe { rn_bmp_cmd_c::bmp_attach_c(target) }
}
/// Write a single target register.
///
/// `reg` is the GDB register number, `value` is the 32-bit value to write.
pub fn bmp_write_register(reg: u32, value: u32) -> bool {
    if let Some(driver) = target_adapter::current_driver() {
        let bytes = value.to_le_bytes();
        driver.reg_write(reg, &bytes).is_ok()
    } else {
        false
    }
}
/// Read a single target register.
///
/// Returns `Some(value)` on success, `None` if the read failed or no target
/// is attached.
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
#[allow(dead_code)]
/// List RISC-V CSRs (Control and Status Registers).

/// Erase a region of target flash memory.
///
/// `adr` is the start address, `size` is the number of bytes to erase.
/// The region must be aligned to flash sector boundaries.
pub fn bmp_flash_erase(adr: u32, size: u32) -> bool {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return false;
    }
    // target_flash_erase returns true on success
    unsafe { flash_adapter::target_flash_erase(cur as *mut _, adr, size as usize) }
}

/// Write data to target flash memory.
///
/// Writes `data` to the target flash at `addr`. The address and length must
/// be aligned to the flash block size. Call `bmp_flash_complete()` after all
/// writes to finalise the programming operation.
pub fn bmp_flash_write(addr: u32, data: &[u8]) -> bool {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return false;
    }
    unsafe { flash_adapter::target_flash_write(cur as *mut _, addr, data.as_ptr(), data.len()) }
}
/// Get the target's driver name.
///
/// Returns the name of the BMP target driver (e.g. "STM32F4", "RP2040").
/// This is strongly linked to the internal BMP target name and is used for
/// target identification in the GDB stub.
pub fn bmp_get_target_name() -> &'static str {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return "";
    }
    unsafe {
        let ptr = target_adapter::target_get_driver_name_c(cur);
        CStr::from_ptr(ptr).to_str().unwrap_or("")
    }
}
/// Write data to target memory (RAM).
///
/// Writes `data` to the target's memory space at `address`. Unlike flash
/// writes, this does not require erase or complete operations.
pub fn bmp_mem_write(address: u32, data: &[u8]) -> bool {
    // Target memory is about to change: the read-ahead line cache is stale.
    crate::mem_cache::invalidate();
    if let Some(driver) = target_adapter::current_driver() {
        driver.mem_write(address, data).is_ok()
    } else {
        false
    }
}

/// Write 32-bit words to target memory.
///
/// Convenience wrapper around `bmp_mem_write` that writes an array of `u32`
/// values. The data is written as raw bytes (4 bytes per word).
pub fn bmp_write_mem32(address: u32, data: &[u32]) -> bool {
    // Target memory is about to change: the read-ahead line cache is stale.
    crate::mem_cache::invalidate();
    let bytes = unsafe {
        core::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4)
    };
    bmp_mem_write(address, bytes)
}
/// Read the target CPUID register.
///
/// Returns the CPUID value which identifies the target's core type and revision.
pub fn bmp_cpuid() -> u32 {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        0
    } else {
        unsafe { target_adapter::target_get_cpuid_c(cur) }
    }
}
/// Finalise a flash programming operation.
///
/// Must be called after all `bmp_flash_write()` calls to complete the
/// programming sequence (e.g. flush caches, verify).
pub fn bmp_flash_complete() -> bool {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return false;
    }
    unsafe { flash_adapter::target_flash_complete(cur as *mut _) }
}

//#[allow(dead_code)]
/// Compute CRC32 of target memory (BMP internal implementation).
///
/// Reads `length` bytes from `address` and computes a CRC32 checksum.
/// Returns `Some(crc)` on success, `None` on failure.
pub fn _bmp_crc32(address: u32, length: u32) -> Option<u32> {
    unsafe {
        let mut crc: u32 = 0;
        let crc_ptr: *mut u32 = &mut crc;
        if rn_bmp_cmd_c::bmp_crc32_c(address, length, crc_ptr) {
            return Some(crc);
        }
        None
    }
}

/// Read target memory into a raw pointer.
///
/// Low-level memory read that writes directly to a C-style pointer.
pub fn bmp_read_mem_ptr(address: u32, size: u32, data: *mut u8) -> bool {
    if data.is_null() {
        return false;
    }
    let slice = unsafe { core::slice::from_raw_parts_mut(data, size as usize) };
    bmp_read_mem(address, slice)
}
/// Ask the target whether memory access requires halting the CPU first.
///
/// This is the generic capability query RTT (and friends) use: it returns
/// true when the target must be stopped for memory I/O, without the caller
/// needing to know which chip/architecture is attached.
pub fn bmp_mem_access_needs_halt() -> bool {
    if let Some(driver) = target_adapter::current_driver() {
        driver.needs_halt_for_mem()
    } else {
        true
    }
}
/// Read target memory without halting the CPU.
///
/// Returns `true` on success, `false` if the target cannot perform non-halting
/// reads (callers should then halt the target and use the regular read path).
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
///
/// Returns `true` on success, `false` if the target cannot perform non-halting
/// writes.
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
///
/// Reads `data.len()` bytes from `address` into the provided buffer.
pub fn bmp_read_mem(address: u32, data: &mut [u8]) -> bool {
    if let Some(driver) = target_adapter::current_driver() {
        driver.mem_read(data, address).is_ok()
    } else {
        false
    }
}
/// Read 32-bit words from target memory.
///
/// Reads `data.len() * 4` bytes from `address` into the provided u32 buffer.
pub fn bmp_read_mem32(address: u32, data: &mut [u32]) -> bool {
    let bytes = unsafe {
        core::slice::from_raw_parts_mut(data.as_mut_ptr() as *mut u8, data.len() * 4)
    };
    bmp_read_mem(address, bytes)
}
/// Enable/disable the per-call memory-access instrumentation log (Phase 0).
///
/// When enabled, every `bmp_mem_read_c`/`bmp_mem_write_c` call emits a
/// `[MEMLOG]` line with (op, addr, len, SWD tx delta, RV tx delta, elapsed µs).
pub fn bmp_set_mem_log(enable: bool) {
    unsafe { rn_bmp_cmd_c::bmp_set_mem_log_c(enable) }
}
// The SDI wire-timing knobs ('mon sdi_wire') are in crate::riscv_extra — see
// the note above bmp_detach().
/// Snapshot the global SWD/RVSWD wire-transaction counters.
///
/// Returns `(swd_tx, rv_tx)` absolute counts since power-up / last reset.
pub fn bmp_mem_counts() -> (u32, u32) {
    unsafe {
        let mut swd: u32 = 0;
        let mut rv: u32 = 0;
        rn_bmp_cmd_c::bmp_mem_counts_c(&mut swd, &mut rv);
        (swd, rv)
    }
}
/// Zero the global SWD/RVSWD wire-transaction counters (Phase 0).
pub fn bmp_mem_counts_reset() {
    unsafe { rn_bmp_cmd_c::bmp_mem_counts_reset_c() }
}
/// Reset the target MCU.
///
/// Asserts the reset line to the target. Returns `true` on success.
pub fn bmp_reset_target() -> bool {
    if let Some(driver) = target_adapter::current_driver() {
        driver.reset();
        true
    } else {
        false
    }
}
/// Set SWD wait states (slows down the SWD clock).
///
/// Adds wait states to the SWD signals to reduce the SWD clock speed.
/// Useful for targets that cannot handle the default speed.
pub fn bmp_set_wait_state(ws: u32) {
    unsafe {
        rn_bmp_cmd_c::bmp_set_wait_state_c(ws);
    }
}
/// Get the current SWD wait state value.
pub fn bmp_get_wait_state() -> u32 {
    unsafe { rn_bmp_cmd_c::bmp_get_wait_state_c() }
}
/// Set the SWD clock frequency.
///
/// Configures the SWD clock to the specified frequency in Hz.
pub fn bmp_set_frequency(fq: u32) {
    unsafe {
        rn_bmp_cmd_c::bmp_set_frequency_c(fq);
    }
}
/// Get the current SWD clock frequency in Hz.
pub fn bmp_get_frequency() -> u32 {
    unsafe { rn_bmp_cmd_c::bmp_get_frequency_c() }
}

/*

typedef enum target_breakwatch {
    TARGET_BREAK_SOFT 0,
    TARGET_BREAK_HARD 1,
    TARGET_WATCH_WRITE 2,
    TARGET_WATCH_READ 3,
    TARGET_WATCH_ACCESS 4,
} target_breakwatch_e;

 */

/// Add a breakpoint or watchpoint on the target.
///
/// `btype` matches the `target_breakwatch` enum from BMP C:
/// - 0 = software breakpoint
/// - 1 = hardware breakpoint
/// - 2 = write watchpoint
/// - 3 = read watchpoint
/// - 4 = access watchpoint
pub fn bmp_add_breakpoint(btype: u32, adr: u32, len: u32) -> bool {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return false;
    }
    unsafe { target_adapter::target_breakwatch_set(cur, btype, adr, len as usize) == 0 }
}
/// Remove a breakpoint or watchpoint from the target.
pub fn bmp_remove_breakpoint(btype: u32, adr: u32, len: u32) -> bool {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return false;
    }
    unsafe { target_adapter::target_breakwatch_clear(cur, btype, adr, len as usize) == 0 }
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
///
/// If `step` is true, the target executes one instruction and halts again.
/// If `step` is false, the target resumes normal execution.
pub fn bmp_halt_resume(step: bool) -> bool {
    if let Some(driver) = target_adapter::current_driver() {
        driver.halt_resume(step);
        true
    } else {
        false
    }
}

/// Poll the target's current state.
///
/// Returns a `HaltState` indicating whether the target is running, halted
/// at a breakpoint, hit a watchpoint, faulted, etc.
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
///
/// Returns a comma-separated string of board names that the firmware
/// was compiled with support for.
pub fn bmp_supported_boards() -> &'static str {
    unsafe {
        let boards = rn_bmp_cmd_c::list_enabled_boards();

        return core::str::from_utf8_unchecked(CStr::from_ptr(boards as *const c_char).to_bytes());
    }
}
/// Get the BMP firmware version string.
pub fn bmp_get_version() -> &'static str {
    unsafe {
        core::str::from_utf8_unchecked(CStr::from_ptr(rn_bmp_cmd_c::bmp_get_version_string() as *const c_char).to_bytes())
    }
}
/// Execute a BMP monitor command.
///
/// Sends a command string to the BMP monitor interface (like the GDB
/// `monitor` command). Returns `true` if the command was accepted.
pub fn bmp_mon(input_as_string: &str) -> bool {
    let mut buf = [0u8; 128];
    let len = core::cmp::min(input_as_string.len(), 127);
    buf[..len].copy_from_slice(&input_as_string.as_bytes()[..len]);
    buf[len] = 0;
    unsafe { rn_bmp_cmd_c::bmp_mon_c(buf.as_ptr()) }
}
/// Get the current free heap size in bytes.
pub fn free_heap() -> u32 {
    unsafe { rn_bmp_cmd_c::free_heap_c() }
}
/// Get the minimum free heap size since boot (watermark).
pub fn min_free_heap() -> u32 {
    unsafe { rn_bmp_cmd_c::min_free_heap_c() }
}

/// Get heap statistics as (min_free, current_free).
pub fn get_heap_stats() -> (u32, u32) {
    (min_free_heap(), free_heap())
}

/*
 pub fn bmp_platform_target_voltage() ->
 {
    unsafe {
        rn_bmp_cmd_c::platform_target_voltage()
    }
 }
*/
/// Check if the attached target is a RISC-V core.
pub fn bmp_is_riscv() -> bool {
    bmp_get_arch() == bmp_arch::BMP_ARCH_RISCV
}
/// Write a CH32V3xx user option byte.
///
/// Configures the CH32V3xx target's user option byte (e.g. memory configuration).
pub fn bmp_ch32v3xx_write_user_option_byte(memory_conf: u8) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_ch32v3xx_write_user_option_byte_c(memory_conf) }
}
/// Read the CH32V3xx user option byte.
pub fn _bmp_ch32v3xx_read_user_option_byte() -> u8 {
    unsafe { rn_bmp_cmd_c::bmp_ch32v3xx_read_user_option_byte_c() }
}
/// Compute CRC32 using the target's hardware CRC unit.
///
/// Uses the target's built-in CRC32 peripheral (if available) for faster
/// checksum computation. Returns `(success, crc_value)`.
pub fn bmp_custom_crc32(address: u32, size_in_bytes: u32) -> (bool, u32) {
    let mut crc: u32 = 0;
    unsafe {
        let r = rn_bmp_cmd_c::bmp_custom_crc32_c(address, size_in_bytes, &mut crc);
        (r, crc)
    }
}
/// Raise a simulated exception for GDB.
///
/// Used to trigger exception handling in the GDB stub for testing or
/// error recovery.
pub fn bmp_raise_exception() {
    unsafe {
        rn_bmp_cmd_c::bmp_raise_exception_c();
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
        rn_bmp_cmd_c::bmp_enable_reset_pin_c(enabled);
    }
}
/// Get the number of available watchpoint and breakpoint slots.
///
/// Returns `(breakpoint_count, watchpoint_count)`.
pub fn bmp_watchpoint_breakpoint_count() -> (u32, u32) {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return (0, 0);
    }
    let mut brk: u32 = 0;
    let mut wtch: u32 = 0;
    unsafe {
        target_adapter::target_breakpoint_watchpoint_count(cur as *mut _, &mut brk, &mut wtch);
    }
    (brk, wtch)
}
/// Check if the target supports hardware breakpoints.
pub fn bmp_has_hw_breakpoint() -> bool {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        false
    } else {
        unsafe { target_adapter::target_has_hw_breakpoint(cur) }
    }
}
/// Check if the target has mass write helpers (for faster flash programming).
pub fn bmp_has_mw_helpers() -> bool {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        false
    } else {
        unsafe { target_adapter::target_has_mw_helpers_c(cur) }
    }
}
/// Overwrite flash memory (erase + write in one operation).
///
/// Combines erase and write into a single operation for targets that support it.
pub fn bmp_overwrite_flash(address: u32, data: &[u8]) -> bool {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        false
    } else {
        unsafe { target_adapter::target_overwrite_flash_c(cur, address, data.as_ptr(), data.len() as u32) }
    }
}
/// Get the mass write page size in bytes.
///
/// Returns the flash page size used by the mass write helper.
pub fn bmp_get_mw_page_size() -> u32 {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        0
    } else {
        unsafe { target_adapter::target_mw_page_size_c(cur) }
    }
}

/// Target architecture enum.
///
/// Identifies whether the attached target is ARM, RISC-V, or unknown.
/// Kept for callers that still want the coarse architecture; RTT no longer
/// uses it (it asks the target whether memory I/O needs halting instead).
#[allow(dead_code)]
#[derive(PartialEq)]
#[repr(u32)]
pub enum bmp_arch {
    BMP_ARCH_NONE = crate::rn_bmp_cmd_c::BMP_ARCH_NONE,
    BMP_ARCH_ARM = crate::rn_bmp_cmd_c::BMP_ARCH_ARM,
    BMP_ARCH_RISCV = crate::rn_bmp_cmd_c::BMP_ARCH_RISCV,
}

/// Reset SWDIO GPIO state (set high + output).
///
/// Drives the SWDIO pin high and configures it as an output. This is used
/// during target detach to release the SWD bus to a known idle state.
#[allow(dead_code)]
pub fn bmp_gpio_reset() {
    unsafe {
        rn_bmp_cmd_c::bmp_gpio_reset();
    }
}

/// Get the target's architecture (ARM or RISC-V).
#[allow(dead_code)]
pub fn bmp_get_arch() -> bmp_arch {
    let cur = target_adapter::get_cur_target();
    if cur.is_null() {
        return bmp_arch::BMP_ARCH_NONE;
    }
    let opts = unsafe { target_adapter::target_get_options_c(cur) };
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
        unsafe { target_adapter::target_get_options_c(cur) }
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
///
/// Returns `Some(value)` on success, `None` if the target is not attached,
/// not RISC-V, or if reading the CSR failed.
pub fn bmp_riscv_read_csr(csr: u32) -> Option<u32> {
    let ctx = crate::riscv::current_riscv_context()?;
    ctx.hart.csr_read(csr as u16).ok()
}

/// Write a RISC-V CSR.
///
/// Returns `true` on success, `false` if failed or not RISC-V.
pub fn bmp_riscv_write_csr(csr: u32, val: u32) -> bool {
    let Some(ctx) = crate::riscv::current_riscv_context() else {
        return false;
    };
    ctx.hart.csr_write(csr as u16, val).is_ok()
}
// EOF


