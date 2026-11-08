//! The probe side of the hosted (PC) build, in Rust.
//!
//! The Blackmagic core compiled for the host still calls the entry points that only
//! a real probe can serve: hardware SPI (`remote_v3_spi_*`), the SWD/ADIv5 tap
//! (`swdptap_init`, `adiv5_swd_*`), the SWJ/BOOT pins (`bmp_pin_*`, `remote_pin_*`,
//! which go out over the remote protocol), RTT, the heap counters and the RISC-V
//! extras. This is the Rust twin of `hosted/src/stubs.cpp`: same symbols, same
//! behaviour - a no-op where the C++ file had one, `do_assert()` where it asserted.
//!
//! Three symbols of that file are deliberately *not* here:
//!   - `ln_set_connected_state()` - `esprit/src/lnSocketRunner.cpp` already defines
//!     it (weak), and only that file calls it.
//!   - `remote_v4_riscv_jtag_dmi_read/write()` - the real ones live in
//!     `blackmagic/src/platforms/hosted/remote/protocol_v4_riscv.c`, which the hosted
//!     link pulls in for them.
//!   - `xAssert(int)` - `esprit/include/lnAssert.h` defines `xAssert` as a macro over
//!     `do_assert()`, so the C++ definition was dead code.

use core::ffi::{c_char, c_int, c_void};

use crate::hosted::transport::{platform_buffer_read, platform_buffer_write_c};

// The remote protocol framing `remote_pin_gen()` / `remote_pin_get()` use
// (blackmagic/src/remote.h, blackmagic/src/platforms/hosted/bmp_remote.h).
const REMOTE_MAX_MSG_SIZE: usize = 1024;
const REMOTE_SOM: u8 = b'!';
const REMOTE_EOM: u8 = b'#';
const REMOTE_GEN_PACKET: u8 = b'G';
const REMOTE_RESP_OK: u8 = b'K';
/// Pin commands: set a pin, set its direction, read it.
const REMOTE_PIN_SET: u8 = b'W';
const REMOTE_PIN_DIRECTION: u8 = b'X';
const REMOTE_PIN_GET: u8 = b'w';

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn putchar(c: c_int) -> c_int;
    fn exit(status: c_int) -> !;
    fn bmp_gpio_reset_c();
}

/// `printf()` with a literal format and no arguments.
fn print_line(line: &'static [u8]) {
    unsafe {
        printf(line.as_ptr() as *const c_char);
    }
}

/// What `xAssert(0)` did in the C++ stubs: `do_assert("0")`, i.e. say so and leave.
fn hosted_assert0() -> ! {
    do_assert(b"0\0".as_ptr() as *const c_char)
}

/// The C++ stubs printed the function name before asserting.
fn unimplemented(name: &'static [u8]) -> ! {
    unsafe {
        printf(b"UNIMPLEMENTED :%s\n\0".as_ptr() as *const c_char, name.as_ptr() as *const c_char);
    }
    hosted_assert0()
}

#[unsafe(no_mangle)]
pub extern "C" fn do_assert(z: *const c_char) -> ! {
    unsafe {
        printf(b"FATAL ERROR :%s\n\0".as_ptr() as *const c_char, z);
        exit(-1)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _putchar(c: c_char) {
    unsafe {
        putchar(c as c_int);
    }
}

// -- SPI: not routed over the remote protocol -------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn remote_v3_spi_init(_bus: i32) -> bool {
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn remote_v3_spi_deinit(_bus: i32) -> bool {
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn remote_v3_spi_chip_select(_device_select: u8) -> bool {
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn remote_v3_spi_xfer(_bus: i32, _value: u8) -> u8 {
    0
}

// -- RTT: nothing to drain on the host --------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn swindle_rtt_send_data_to_host(_index: u32, _len: u32, _data: *const u8) {}

#[unsafe(no_mangle)]
pub extern "C" fn swindle_rtt_room_available_to_host(_index: u32) -> u32 {
    0
}

// -- Session / reset knobs the host does not drive --------------------------

#[unsafe(no_mangle)]
pub extern "C" fn platform_nrst_set_val_internal(_assert: bool) {}

#[unsafe(no_mangle)]
pub extern "C" fn platform_nrst_set_val(assert: bool) {
    crate::hosted::rpc_host_generated::remote_gen_nrst_set(assert);
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_nrst_get_val() -> bool {
    crate::hosted::rpc_host_generated::remote_gen_nrst_get() != 0
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_delay(ms: u32) {
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_target_voltage() -> *const c_char {
    static mut VOLT_BUF: [u8; 32] = [0u8; 32];
    // Already decoded from the CBOR reply, empty when the probe failed.
    let v = crate::hosted::rpc_host_generated::remote_gen_voltage();
    if !v.is_empty() {
        let copy_len = v.len().min(31);
        unsafe {
            let buf = &mut *core::ptr::addr_of_mut!(VOLT_BUF);
            buf[..copy_len].copy_from_slice(&v[..copy_len]);
            buf[copy_len] = 0;
            buf.as_ptr() as *const c_char
        }
    } else {
        b"unknown\0".as_ptr() as *const c_char
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn platform_target_clk_output_enable(enable: bool) {
    crate::hosted::rpc_host_generated::remote_gen_target_clk_oe(enable);
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_mem_counts_c(swd: *mut u32, rv: *mut u32) {
    unsafe {
        if !swd.is_null() {
            *swd = 0;
        }
        if !rv.is_null() {
            *rv = 0;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_mem_counts_reset_c() {}

#[unsafe(no_mangle)]
pub extern "C" fn min_free_heap_c() -> usize {
    2
}

#[unsafe(no_mangle)]
pub extern "C" fn free_heap_c() -> usize {
    33
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_raise_exception_c() -> ! {
    panic!("BMP exception");
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_tweak_options(_opt: *mut u32) {}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_get_temporary_buffer(asked: u32) -> *mut c_void {
    static mut BUFFER: [u8; 4096] = [0u8; 4096];
    assert!(asked as usize <= 4096);
    core::ptr::addr_of_mut!(BUFFER) as *mut c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_log_mem_c(_is_write: bool, _addr: u32, _length: u32) {}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_debug_warn_c(msg: *const c_char) {
    unsafe {
        if !msg.is_null() {
            printf(b"WARN: %s\n\0".as_ptr() as *const c_char, msg);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_io_begin_session() {}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_io_end_session() {}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_get_version_string() -> *const c_char {
    b"swindle hosted\0".as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_set_wait_state_c(ws: u32) {
    unsafe {
        printf(b"Stubbed : set ws %d\n\0".as_ptr() as *const c_char, ws);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_get_wait_state_c() -> u32 {
    let ws = 5;
    unsafe {
        printf(b"Stubbed : get ws %d\n\0".as_ptr() as *const c_char, ws);
    }
    ws
}


// -- ADIv5 SWD tap ---------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn adiv5_swd_write_no_check(_addr: u16, _data: u32) -> bool {
    unimplemented(b"adiv5_swd_write_no_check\0")
}

#[unsafe(no_mangle)]
pub extern "C" fn adiv5_swd_read_no_check(_addr: u16) -> u32 {
    unimplemented(b"adiv5_swd_read_no_check\0")
}

#[unsafe(no_mangle)]
pub extern "C" fn swdptap_init() {
    crate::hosted::rpc_host_generated::remote_swdp_init();
}

#[unsafe(no_mangle)]
pub extern "C" fn list_enabled_boards() -> *const c_char {
    c"CH32F1, RP2040, RP2350, STM32F1, STM32F4, STM32G0".as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn stlink_adiv5_dp_init() {
    hosted_assert0()
}

#[unsafe(no_mangle)]
pub extern "C" fn x_nrf51_ctrl_ap_probe(_ap: *mut c_void) -> bool {
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn dap_run_cmd(
    _request_data: *const c_void,
    _request_length: usize,
    _response_data: *mut c_void,
    _response_length: usize,
) -> bool {
    hosted_assert0()
}

// -- Pins: forwarded to the probe over the remote protocol -----------------

/// Send a pin command and report whether the probe answered OK.
fn remote_pin_gen(code: u8, pin: u8, value: u8) {
    let mut buffer = [0u8; REMOTE_MAX_MSG_SIZE];
    buffer[0] = REMOTE_SOM;
    buffer[1] = REMOTE_GEN_PACKET;
    buffer[2] = code;
    buffer[3] = pin;
    buffer[4] = value;
    buffer[5] = REMOTE_EOM;
    unsafe {
        platform_buffer_write_c(buffer.as_ptr(), 6);
        let length = platform_buffer_read(buffer.as_mut_ptr(), REMOTE_MAX_MSG_SIZE as c_int);
        if length < 1 {
            print_line(b"Invalid pin set reply\n\0");
            return;
        }
        if buffer[0] != REMOTE_RESP_OK {
            print_line(b" pin set error\n\0");
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_pin_set(pin: u8, state: u8) {
    remote_pin_set(pin, state);
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_pin_get(pin: u8) -> bool {
    remote_pin_get(pin)
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_pin_direction(pin: u8, is_write: u8) {
    remote_pin_direction(pin, is_write);
}

#[unsafe(no_mangle)]
pub extern "C" fn remote_pin_direction(pin: u8, is_write: u8) {
    remote_pin_gen(REMOTE_PIN_DIRECTION, pin, is_write);
}

/// `pin` 0 is the clock line, 1 the data line.
#[unsafe(no_mangle)]
pub extern "C" fn remote_pin_set(pin: u8, value: u8) {
    remote_pin_gen(REMOTE_PIN_SET, pin, value);
}

#[unsafe(no_mangle)]
pub extern "C" fn remote_pin_get(pin: u8) -> bool {
    let mut buffer = [0u8; REMOTE_MAX_MSG_SIZE];
    buffer[0] = REMOTE_SOM;
    buffer[1] = REMOTE_GEN_PACKET;
    buffer[2] = REMOTE_PIN_GET;
    buffer[3] = pin;
    buffer[4] = REMOTE_EOM;
    unsafe {
        platform_buffer_write_c(buffer.as_ptr(), 5);
        let length = platform_buffer_read(buffer.as_mut_ptr(), REMOTE_MAX_MSG_SIZE as c_int);
        if length < 1 {
            print_line(b"Invalid pin set reply\n\0");
            return false;
        }
        if buffer[0] != REMOTE_RESP_OK {
            print_line(b" pin set error\n\0");
            hosted_assert0();
        }
        // The probe answers the level as a character.
        match buffer[2] {
            b'0' => false,
            b'1' => true,
            _ => hosted_assert0(),
        }
    }
}

// -- RISC-V / RVSWD extras -------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn riscv_jtag_dtm_handler(_dev_index: u8) {}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_rv_dm_read_c(_adr: u8, _value: *mut u32) -> bool {
    hosted_assert0()
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_rv_dm_write_c(_adr: u8, _value: u32) -> bool {
    hosted_assert0()
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_rv_dm_reset_c() -> bool {
    hosted_assert0()
}

#[unsafe(no_mangle)]
pub extern "C" fn remote_v4_adiv6_ap_read() {
    hosted_assert0()
}

#[unsafe(no_mangle)]
pub extern "C" fn remote_v4_adiv6_ap_write() {
    hosted_assert0()
}

#[unsafe(no_mangle)]
pub extern "C" fn remote_v4_adiv6_mem_read_bytes() {
    hosted_assert0()
}

#[unsafe(no_mangle)]
pub extern "C" fn remote_v4_adiv6_mem_write_bytes() {
    hosted_assert0()
}

// -- Probe-side services the host has no hardware for ----------------------

#[unsafe(no_mangle)]
pub extern "C" fn bmp_get_target_voltage_c() -> f32 {
    0.0
}

#[unsafe(no_mangle)]
pub extern "C" fn setLogger(_logger: *mut c_void) {}

#[unsafe(no_mangle)]
pub extern "C" fn usbCdc_Logger(_n: i32, _data: *const c_char) {}

#[unsafe(no_mangle)]
pub extern "C" fn lnGetFreeRTOSDebug() -> *const u32 {
    core::ptr::null()
}

#[unsafe(no_mangle)]
pub extern "C" fn xPortGetFreeHeapSize() -> usize {
    33
}

#[unsafe(no_mangle)]
pub extern "C" fn xPortGetMinimumEverFreeHeapSize() -> usize {
    2
}

#[unsafe(no_mangle)]
pub extern "C" fn gdb_if_init() -> c_int {
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn bmp_gpio_reset() {
    unsafe {
        bmp_gpio_reset_c();
    }
}

/// The probe's own firmware defines `lnSoftSystemReset()` in C++ (the esprit MCU
/// layer), which is why `commands/mon.rs` calls it under its mangled name; a hosted
/// build has nothing to reset, so this is the Rust twin of the C++ stub's empty body,
/// exported the same way.
#[unsafe(export_name = "_Z17lnSoftSystemResetv")]
pub extern "C" fn ln_soft_system_reset() {}
// EOF
