//! BMP C FFI bindings — safe Rust wrappers around the Black Magic Probe C library.
//!
//! Each function wraps a corresponding `rn_bmp_cmd_c` C call, converting between
//! Rust-safe types and C pointers. The BMP C library owns the actual debug hardware
//! access (SWD bit-banging, target memory read/write, flash programming).


use crate::commands::run::HaltState;
use crate::rn_bmp_cmd_c;
use alloc::vec::Vec;
use core::ffi::CStr;
use core::ptr::null;
use core::ptr::null_mut;

#[cfg(feature = "hosted")]
type my_c_str = *const i8;
#[cfg(not(feature = "hosted"))]
type my_c_str = *const u8;

pub enum mapping {
    Flash = 0,
    Ram = 1,
}

pub struct MemoryBlock {
    pub start_address: u32,
    pub length: u32,
    pub block_size: u32,
}

/// Get the XML target register description from BMP.
///
/// Returns a string containing the GDB XML register description for the
/// currently attached target. This is used by GDB to understand the register
/// set (names, types, groups).
pub fn bmp_register_description() -> &'static str {
    //
    unsafe {
        #![allow(clippy::manual_unwrap_or_default)]
        match CStr::from_ptr(rn_bmp_cmd_c::bmp_target_description_c() as my_c_str).to_str() {
            Ok(x) => x,
            Err(_y) => "",
        }
    }
}
/// Free the register description string allocated by BMP.
///
/// Must be called after `bmp_register_description()` to avoid leaking the
/// C-allocated string buffer.
pub fn bmp_drop_register_description() {
    unsafe { rn_bmp_cmd_c::bmp_target_description_clear_c() }
}

/// Read all target registers into a vector.
///
/// Returns a `Vec<u32>` containing all registers in the order defined by the
/// target's GDB register description. Returns an empty vector if no target
/// is attached.
pub fn bmp_read_registers() -> Vec<u32> {
    if !bmp_attached() {
        let r: Vec<u32> = Vec::new();
        return r;
    }
    unsafe {
        let n = rn_bmp_cmd_c::bmp_registers_count_c() as usize;
        let mut r: Vec<u32> = vec![0; n];
        rn_bmp_cmd_c::bmp_read_all_registers_c(r.as_mut_ptr());
        r
    }
}
/// Get the target's memory map (flash or RAM regions).
///
/// Queries the BMP target for its flash or RAM memory map and returns a
/// vector of `MemoryBlock` descriptors. Each block describes a contiguous
/// region with its start address, length, and (for flash) block size.
pub fn bmp_get_mapping(map: mapping) -> Vec<MemoryBlock> {
    if !bmp_attached() {
        let r: Vec<MemoryBlock> = Vec::new();
        return r;
    }
    let mut r: Vec<MemoryBlock> = Vec::new();
    let count: usize;
    let imap = map as u32;
    unsafe {
        count = rn_bmp_cmd_c::bmp_map_count_c(imap) as usize;
    };
    if count != 0 {
        unsafe {
            let mut start: u32 = 0;
            let mut size: u32 = 0;
            let mut block_size: u32 = 0;
            for i in 0..count {
                if rn_bmp_cmd_c::bmp_map_get_c(
                    imap,
                    i as u32,
                    &mut start as *mut u32,
                    &mut size as *mut u32,
                    &mut block_size as *mut u32,
                ) {
                    r.push(MemoryBlock {
                        start_address: start,
                        length: size,
                        block_size,
                    });
                }
            }
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
/// Detach from the currently attached target.
///
/// Releases the target, disables RTT, and resets the GPIO state. Returns
/// `true` on success.
pub fn bmp_detach() -> bool {
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
    unsafe { rn_bmp_cmd_c::bmp_attached_c() }
}
/// Attach to a target by index.
///
/// `target` is the zero-based index from the SWD scan results. Returns `true`
/// if attachment succeeded.
pub fn bmp_attach(target: u32) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_attach_c(target) }
}
/// Write a single target register.
///
/// `reg` is the GDB register number, `value` is the 32-bit value to write.
pub fn bmp_write_register(reg: u32, value: u32) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_write_reg_c(reg, value) }
}
/// Read a single target register.
///
/// Returns `Some(value)` on success, `None` if the read failed or no target
/// is attached.
pub fn bmp_read_register(reg: u32) -> Option<u32> {
    let mut val: u32 = 0;
    unsafe {
        let ptr: *mut u32 = &mut val;
        if rn_bmp_cmd_c::bmp_read_reg_c(reg, ptr) {
            return Some(val);
        }
        None
    }
}
#[allow(dead_code)]
/// List RISC-V CSRs (Control and Status Registers).
///
/// Queries the BMP for the list of available RISC-V CSRs. Writes up to
/// `out.len()` CSR numbers into the provided buffer. Returns `None` if the
/// buffer is too small.
pub fn riscv_list_csr(out: &mut [u32]) -> Option<&[u32]> {
    let n = unsafe { rn_bmp_cmd_c::riscv_list_csr(0, 0, core::ptr::null_mut()) };
    if n > (out.len() as u32) {
        return None;
    }
    let out_u32: *mut u32 = &mut out[0];
    let r: usize = unsafe { rn_bmp_cmd_c::riscv_list_csr(0, n, out_u32) as usize };
    Some(&out[0..r])
}

/// Erase a region of target flash memory.
///
/// `adr` is the start address, `size` is the number of bytes to erase.
/// The region must be aligned to flash sector boundaries.
pub fn bmp_flash_erase(adr: u32, size: u32) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_flash_erase_c(adr, size) }
}

/// Write data to target flash memory.
///
/// Writes `data` to the target flash at `addr`. The address and length must
/// be aligned to the flash block size. Call `bmp_flash_complete()` after all
/// writes to finalise the programming operation.
pub fn bmp_flash_write(addr: u32, data: &[u8]) -> bool {
    unsafe {
        let ptr: *const u8 = data.as_ptr();
        rn_bmp_cmd_c::bmp_flash_write_c(addr, data.len() as u32, ptr)
    }
}
/// Get the target's driver name.
///
/// Returns the name of the BMP target driver (e.g. "STM32F4", "RP2040").
/// This is strongly linked to the internal BMP target name and is used for
/// target identification in the GDB stub.
pub fn bmp_get_target_name() -> &'static str {
    unsafe {
        #![allow(clippy::manual_unwrap_or_default)]
        match CStr::from_ptr(rn_bmp_cmd_c::bmp_get_driver_name_c() as my_c_str).to_str() {
            Ok(x) => x,
            _ => "",
        }
    }
}
/// Write data to target memory (RAM).
///
/// Writes `data` to the target's memory space at `address`. Unlike flash
/// writes, this does not require erase or complete operations.
pub fn bmp_mem_write(address: u32, data: &[u8]) -> bool {
    unsafe {
        let ptr: *const u8 = data.as_ptr();
        rn_bmp_cmd_c::bmp_mem_write_c(address, data.len() as u32, ptr)
    }
}

/// Write 32-bit words to target memory.
///
/// Convenience wrapper around `bmp_mem_write` that writes an array of `u32`
/// values. The data is written as raw bytes (4 bytes per word).
pub fn bmp_write_mem32(address: u32, data: &[u32]) -> bool {
    unsafe {
        // mem_read_c returns flase if ok (WTF)
        rn_bmp_cmd_c::bmp_mem_write_c(address, (data.len() as u32) * 4, data.as_ptr() as *const u8)
    }
}
/// Read the target CPUID register.
///
/// Returns the CPUID value which identifies the target's core type and revision.
pub fn bmp_cpuid() -> u32 {
    unsafe { rn_bmp_cmd_c::bmp_get_cpuid_c() }
}
/// Finalise a flash programming operation.
///
/// Must be called after all `bmp_flash_write()` calls to complete the
/// programming sequence (e.g. flush caches, verify).
pub fn bmp_flash_complete() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_flash_complete_c() }
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
/// Returns `true` on success (note: the BMP C function returns false for OK).
pub fn bmp_read_mem_ptr(address: u32, size: u32, data: *mut u8) -> bool {
    unsafe {
        // mem_read_c returns flase if ok (WTF)
        rn_bmp_cmd_c::bmp_mem_read_c(address, size, data)
    }
}
/// Read target memory into a byte slice.
///
/// Reads `data.len()` bytes from `address` into the provided buffer.
/// Returns `true` on success (note: the BMP C function returns false for OK).
pub fn bmp_read_mem(address: u32, data: &mut [u8]) -> bool {
    unsafe {
        // mem_read_c returns flase if ok (WTF)
        rn_bmp_cmd_c::bmp_mem_read_c(address, data.len() as u32, data.as_mut_ptr())
    }
}
/// Read 32-bit words from target memory.
///
/// Reads `data.len() * 4` bytes from `address` into the provided u32 buffer.
/// Returns `true` on success (note: the BMP C function returns false for OK).
pub fn bmp_read_mem32(address: u32, data: &mut [u32]) -> bool {
    unsafe {
        // mem_read_c returns flase if ok (WTF)
        rn_bmp_cmd_c::bmp_mem_read_c(
            address,
            (data.len() as u32) * 4,
            data.as_mut_ptr() as *mut u8,
        )
    }
}
/// Reset the target MCU.
///
/// Asserts the reset line to the target. Returns `true` on success.
pub fn bmp_reset_target() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_reset_target_c() }
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
    unsafe { rn_bmp_cmd_c::bmp_add_breakpoint_c(btype, adr, len) }
}
/// Remove a breakpoint or watchpoint from the target.
pub fn bmp_remove_breakpoint(btype: u32, adr: u32, len: u32) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_remove_breakpoint_c(btype, adr, len) }
}

/// Halt the target MCU execution.
pub fn bmp_target_halt() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_target_halt_c() }
}

/// Resume target execution, optionally stepping a single instruction.
///
/// If `step` is true, the target executes one instruction and halts again.
/// If `step` is false, the target resumes normal execution.
pub fn bmp_halt_resume(step: bool) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_target_halt_resume_c(step) }
}

/// Poll the target's current state.
///
/// Returns a `HaltState` indicating whether the target is running, halted
/// at a breakpoint, hit a watchpoint, faulted, etc.
pub fn bmp_poll() -> HaltState {
    unsafe {
        let mut wp: u32 = 0;
        let wp_ptr: *mut u32 = &mut wp;
        let r: u32 = rn_bmp_cmd_c::bmp_poll_target_c(wp_ptr);
        match r {
            0 => HaltState::Running,
            1 => HaltState::Error,
            2 => HaltState::Request,
            3 => HaltState::Stepping,
            4 => HaltState::Breakpoint,
            5 => HaltState::Watchpoint(wp),
            6 => HaltState::Fault,
            _ => panic!("wrong halt reason"),
        }
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

        let output = CStr::from_ptr(boards as my_c_str).to_str();
        if let Ok(x) = output {
            return x;
        }
    }
    "--error--"
}
/// Get the BMP firmware version string.
pub fn bmp_get_version() -> &'static str {
    unsafe {
        match CStr::from_ptr(rn_bmp_cmd_c::bmp_get_version_string() as my_c_str).to_str() {
            Ok(x) => x,
            _ => "??",
        }
    }
}
/// Execute a BMP monitor command.
///
/// Sends a command string to the BMP monitor interface (like the GDB
/// `monitor` command). Returns `true` if the command was accepted.
pub fn bmp_mon(input_as_string: &str) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_mon_c(input_as_string.as_ptr()) }
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
    unsafe { rn_bmp_cmd_c::bmp_is_riscv_c() }
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
/// Begin a try block for BMP exception handling.
///
/// Returns `true` if no exception is pending. Used with `bmp_catch()` to
/// detect stray exceptions during GDB command execution.
pub fn bmp_try() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_try_c() }
}
/// End a try block and check for BMP exceptions.
///
/// Returns non-zero if an exception occurred since the last `bmp_try()` call.
pub fn bmp_catch() -> i32 {
    unsafe { rn_bmp_cmd_c::bmp_catch_c() }
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
    unsafe {
        let mut brk: u32 = 0;
        let mut wtch: u32 = 0;
        rn_bmp_cmd_c::bmp_breakpoint_watchpoint_count_c(&mut brk, &mut wtch);
        (brk, wtch)
    }
}
/// Check if the target supports hardware breakpoints.
pub fn bmp_has_hw_breakpoint() -> bool {
    unsafe { rn_bmp_cmd_c::bmp_has_hw_breakpoint_c() }
}
/// Check if the target has mass write helpers (for faster flash programming).
pub fn bmp_has_mw_helpers() -> bool {
    unsafe { rn_bmp_cmd_c::target_has_mw_helpers() }
}
/// Overwrite flash memory (erase + write in one operation).
///
/// Combines erase and write into a single operation for targets that support it.
pub fn bmp_overwrite_flash(address: u32, data: &[u8]) -> bool {
    unsafe { rn_bmp_cmd_c::bmp_overwrite_flash_c(address, data.as_ptr(), data.len() as u32) }
}
/// Get the mass write page size in bytes.
///
/// Returns the flash page size used by the mass write helper.
pub fn bmp_get_mw_page_size() -> u32 {
    unsafe { rn_bmp_cmd_c::target_mw_page_size() }
}

/// Target architecture enum.
///
/// Identifies whether the attached target is ARM, RISC-V, or unknown.
#[derive(PartialEq)]
#[repr(u32)]
pub enum bmp_arch {
    BMP_ARCH_NONE = crate::rn_bmp_cmd_c::BMP_ARCH_NONE,
    BMP_ARCH_ARM = crate::rn_bmp_cmd_c::BMP_ARCH_ARM,
    BMP_ARCH_RISCV = crate::rn_bmp_cmd_c::BMP_ARCH_RISCV,
}

/// Get the target's architecture (ARM or RISC-V).
pub fn bmp_get_arch() -> bmp_arch {
    unsafe {
        match rn_bmp_cmd_c::bmp_get_arch_c() {
            crate::rn_bmp_cmd_c::BMP_ARCH_NONE => bmp_arch::BMP_ARCH_NONE,
            crate::rn_bmp_cmd_c::BMP_ARCH_ARM => bmp_arch::BMP_ARCH_ARM,
            crate::rn_bmp_cmd_c::BMP_ARCH_RISCV => bmp_arch::BMP_ARCH_RISCV,
            _ => bmp_arch::BMP_ARCH_NONE,
        }
    }
}
// EOF
