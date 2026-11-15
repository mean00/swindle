//! Rust-owned logger CDC (3rd CDC instance).
//!
//! Replaces the C++ `BMPUsbLogger` class in `bmp_cdc_logger.cpp`.
//! Provides a dedicated CDC‑ACM virtual COM port for debug log output.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::SyncCell;
use rust_esprit::{Cdc, CdcEvent, CdcEventHandler, Mutex};

/// Maximum time `UsbLogger::write` waits for the host to drain CDC 2 before dropping
/// the remaining bytes so RTT output cannot wedge `gdb_task` when no terminal is attached.
const LOGGER_WRITE_TIMEOUT_MS: u32 = 250;

// ---------------------------------------------------------------------------
//  Logger CDC event handler
// ---------------------------------------------------------------------------

/// Handler that implements `CdcEventHandler` for the logger CDC.
struct LoggerCdcHandler;

impl CdcEventHandler for LoggerCdcHandler {
    fn handle(&self, _interface: i32, event: CdcEvent, _payload: u32) {
        if event == CdcEvent::SessionStart {
            SHARED.connected.store(true, Ordering::Relaxed);
        }
    }
}

static LOGGER_HANDLER: LoggerCdcHandler = LoggerCdcHandler;

// ---------------------------------------------------------------------------
//  UsbLogger
// ---------------------------------------------------------------------------

struct LoggerShared {
    cdc: SyncCell<core::mem::MaybeUninit<Cdc>>,
    connected: AtomicBool,
}

// SAFETY: `connected` is atomic and `cdc` is initialized once before use and
// delegates to the underlying C CDC driver.
unsafe impl Sync for LoggerShared {}

impl LoggerShared {
    #[inline(always)]
    fn cdc(&self) -> &Cdc {
        // SAFETY: Initialized in `UsbLogger::init` before any write/write_available call.
        unsafe { (*self.cdc.as_ptr()).assume_init_ref() }
    }
}

static SHARED: LoggerShared = LoggerShared {
    cdc: SyncCell::new(core::mem::MaybeUninit::uninit()),
    connected: AtomicBool::new(false),
};
static LOGGER_CDC_INITIALIZED: AtomicBool = AtomicBool::new(false);
/// Serializes TX FIFO writes between `gdb_task` (`UsbLogger::write`) and log callers
/// (`UsbLogger::write_no_block`).
static TX_MUTEX: Mutex<()> = Mutex::new(());

/// Simple logger that writes to a dedicated CDC instance.
pub struct UsbLogger;

impl UsbLogger {
    /// Initialise the logger CDC on `instance`.
    pub fn init(instance: u32) {
        if LOGGER_CDC_INITIALIZED.load(Ordering::Acquire) {
            return;
        }

        unsafe {
            (*SHARED.cdc.as_ptr()).write(Cdc::new_static(instance, &LOGGER_HANDLER));
        }

        LOGGER_CDC_INITIALIZED.store(true, Ordering::Release);
    }

    /// Write data to the logger CDC (bounded by `LOGGER_WRITE_TIMEOUT_MS`).
    pub fn write(data: &[u8]) {
        let _guard = TX_MUTEX.lock();
        let cdc = SHARED.cdc();
        let n = cdc.write_timeout(data, LOGGER_WRITE_TIMEOUT_MS);
        if n > 0 {
            cdc.flush_once();
        }
    }

    /// Non-blocking variant: whatever fits in the CDC FIFO, the rest is dropped.
    pub fn write_no_block(data: &[u8]) {
        let Some(_guard) = TX_MUTEX.try_lock() else {
            return;
        };
        let cdc = SHARED.cdc();
        let n = cdc.write_no_block(data);
        if n > 0 {
            cdc.flush_once();
        }
    }

    /// Returns the number of bytes available for writing.
    pub fn write_available() -> u32 {
        if !LOGGER_CDC_INITIALIZED.load(Ordering::Acquire) {
            return 0;
        }
        SHARED.cdc().write_available() as u32
    }
}

// ---------------------------------------------------------------------------
//  C-compatible exports
// ---------------------------------------------------------------------------

/// Initialise the logger CDC (called from C++ `initCDCLogger` equivalent).
#[unsafe(no_mangle)]
pub extern "C" fn rn_logger_cdc_init(instance: u32) {
    UsbLogger::init(instance);
}

/// Write to the logger CDC (called from C code via `usbCdc_Logger`).
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rn_usb_cdc_logger(n: i32, data: *const u8) {
    if n <= 0 || !LOGGER_CDC_INITIALIZED.load(Ordering::Acquire) {
        return;
    }
    let slice = unsafe { core::slice::from_raw_parts(data, n as usize) };
    UsbLogger::write(slice);
}

/// Non-blocking write for the `mon redirect` log sink - see
/// `rn_serial_bridge_write_nb` for why the sink must not block.
#[unsafe(no_mangle)]
pub extern "C" fn rn_usb_cdc_logger_nb(n: i32, data: *const u8) {
    if n <= 0 || !LOGGER_CDC_INITIALIZED.load(Ordering::Acquire) {
        return;
    }
    let slice = unsafe { core::slice::from_raw_parts(data, n as usize) };
    UsbLogger::write_no_block(slice);
}

/// Returns write space available on the logger CDC.
#[unsafe(no_mangle)]
pub extern "C" fn rn_usb_cdc_write_available() -> u32 {
    if !LOGGER_CDC_INITIALIZED.load(Ordering::Acquire) {
        return 0;
    }
    UsbLogger::write_available()
}

// EOF
