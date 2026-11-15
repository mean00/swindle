//! GD32F303 board entry (`swindle_gd32_ws2812.cpp`) and USB GDB task (`bmp_usb.cpp`)
//! in Rust (`drop_c_v4.md` Phase 3 / Step 2).

use core::ffi::{c_char, c_void};

use rs_swindle_board::BOARD;
use rust_esprit::task::TaskStorage;

const TASK_BMP_SERIAL_PRIORITY: u32 = 4;
const TASK_BMP_GDB_PRIORITY: u32 = 5;
const TASK_BMP_SERIAL_STACK_WORDS: usize = (2 * 1024) / 4;
const TASK_BMP_GDB_STACK_WORDS: usize = (4 * 1024) / 4;

const USB_EV_CONNECT: i32 = 0;
const USB_EV_DISCONNECT: i32 = 1;
const USB_EV_SUSPEND: i32 = 2;
const USB_EV_RESUME: i32 = 3;

static GDB_TASK_STORAGE: TaskStorage<TASK_BMP_GDB_STACK_WORDS> = TaskStorage::new();
static USBSERIAL_TASK_STORAGE: TaskStorage<TASK_BMP_SERIAL_STACK_WORDS> = TaskStorage::new();

unsafe extern "C" {
    fn Logger(fmt: *const c_char, ...);
    fn swindle_leds_run() -> !;
    #[link_name = "\u{1}_Z18bmp_gpio_init_oncev"]
    fn bmp_gpio_init_once();
    fn platform_init();
    fn pins_init();
    fn swindle_init_rtt();
    fn lnUsbDcdInit();
    fn rn_usb_stack_init(
        device: *const c_void,
        qualifier: *const c_void,
        fs_config: *const u8,
        hs_config: *const u8,
        strings: *const *const c_char,
        string_count: u32,
    );
    fn rn_usb_stack_set_event_handler(
        cookie: *mut c_void,
        cb: extern "C" fn(*mut c_void, i32),
    );
    fn rn_usb_stack_start();
    fn rn_usb_dfu_init();
    fn rngdb_cdc_init(instance: u32);
    fn rngdb_gdb_task();
    #[cfg(feature = "cdc3")]
    fn rn_logger_cdc_init(instance: u32);
    #[cfg(feature = "cdc3")]
    fn rn_usb_cdc_logger(n: i32, data: *const u8);
    #[cfg(feature = "cdc3")]
    fn rn_usb_cdc_write_available() -> u32;
    fn rn_serial_bridge_init(usb_instance: u32, serial_instance: i32, baud: i32);
    fn rn_serial_bridge_task(_param: *mut c_void);
    #[cfg(not(feature = "cdc3"))]
    fn rn_serial_bridge_write(n: i32, data: *const u8);

    static desc_device: [u8; 18];
    static desc_device_qualifier: [u8; 10];
    static desc_fs_configuration: [u8; 1];
    static desc_hs_configuration: [u8; 1];
    static descriptor: [*const c_char; 1];
    static bmp_usb_descriptor_count: u32;
}

extern "C" fn bridge_task_entry(_param: *mut c_void) {
    // SAFETY: `rn_serial_bridge_task` is the Rust bridge task loop (never returns).
    unsafe { rn_serial_bridge_task(core::ptr::null_mut()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn swindle_rtt_send_data_to_host(_index: u32, len: u32, data: *const u8) {
    #[cfg(feature = "cdc3")]
    unsafe {
        rn_usb_cdc_logger(len as i32, data);
    }
    #[cfg(not(feature = "cdc3"))]
    unsafe {
        rn_serial_bridge_write(len as i32, data);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn swindle_rtt_room_available_to_host(_dex: u32) -> u32 {
    #[cfg(feature = "cdc3")]
    unsafe {
        rn_usb_cdc_write_available()
    }
    #[cfg(not(feature = "cdc3"))]
    {
        64
    }
}

extern "C" fn hello_usb_event(_cookie: *mut c_void, event: i32) {
    match event {
        USB_EV_CONNECT | USB_EV_DISCONNECT | USB_EV_SUSPEND | USB_EV_RESUME => {}
        _ => panic!("unknown USB event"),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gdb_if_init() -> bool {
    // SAFETY: All FFI calls and descriptor statics are valid for the lifetime of the probe.
    unsafe {
        Logger(c"Starting CDC \n".as_ptr());
        lnUsbDcdInit();
        rn_usb_stack_init(
            desc_device.as_ptr().cast(),
            desc_device_qualifier.as_ptr().cast(),
            desc_fs_configuration.as_ptr(),
            desc_hs_configuration.as_ptr(),
            descriptor.as_ptr(),
            bmp_usb_descriptor_count,
        );
        rn_usb_stack_set_event_handler(core::ptr::null_mut(), hello_usb_event);
        rngdb_cdc_init(0);
        #[cfg(feature = "cdc3")]
        rn_logger_cdc_init(BOARD.usb_logger);
        rn_serial_bridge_init(BOARD.usb_gdb, BOARD.usb_serial as i32, 115200);
    }

    USBSERIAL_TASK_STORAGE.spawn(
        "usbserial",
        TASK_BMP_SERIAL_PRIORITY,
        bridge_task_entry,
        core::ptr::null_mut(),
    );

    // SAFETY: USB stack and DFU runtime initialization.
    unsafe {
        rn_usb_dfu_init();
        rn_usb_stack_start();
    }
    true
}

extern "C" fn gdb_task(_param: *mut c_void) {
    // SAFETY: Runs once on the dedicated `gdbTask` FreeRTOS task.
    unsafe {
        Logger(c"Gdb task starting... \n".as_ptr());
        platform_init();
        pins_init();
        gdb_if_init();
        swindle_init_rtt();
        Logger(c"Here we go... \n".as_ptr());
        rngdb_gdb_task();
    }
    panic!("rngdb_gdb_task returned");
}

#[unsafe(export_name = "_Z24debug_serial_send_stdoutPKhj")]
pub extern "C" fn debug_serial_send_stdout(data: *const u8, _len: usize) {
    // SAFETY: Caller passes a valid C string pointer.
    unsafe { Logger(c"%s".as_ptr(), data) };
}

#[unsafe(no_mangle)]
pub extern "C" fn user_init() {
    // SAFETY: Initializes probe GPIO once and spawns `gdbTask`.
    unsafe {
        Logger(c"Starting Swindle (GD32)\n".as_ptr());
        bmp_gpio_init_once();
    }
    GDB_TASK_STORAGE.spawn(
        "gdbTask",
        TASK_BMP_GDB_PRIORITY,
        gdb_task,
        core::ptr::null_mut(),
    );
}

#[unsafe(export_name = "_Z5setupv")]
pub extern "C" fn setup() {}

#[unsafe(export_name = "_Z4loopv")]
pub extern "C" fn loop_entry() {
    // SAFETY: Calls `user_init` then enters the never-returning LED loop.
    unsafe {
        Logger(c"Starting Swindle\n".as_ptr());
        user_init();
        swindle_leds_run();
    }
}
