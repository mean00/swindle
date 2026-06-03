//! Rust-owned logger CDC (3rd CDC instance).
//!
//! Replaces the C++ `BMPUsbLogger` class in `bmp_cdc_logger.cpp`.
//! Provides a dedicated CDC‑ACM virtual COM port for debug log output.

use alloc::boxed::Box;
use core::sync::atomic::{AtomicBool, Ordering};

use rust_esprit::cdc::{CdcAcm, CdcEvent, CdcEventHandler};

// ---------------------------------------------------------------------------
//  Logger CDC event handler
// ---------------------------------------------------------------------------

/// Thin handler that implements `CdcEventHandler` for the logger CDC.
struct LoggerCdcHandler {
    logger: *mut UsbLogger,
}

impl CdcEventHandler for LoggerCdcHandler {
    fn handle(&self, _interface: i32, event: CdcEvent, _payload: u32) {
        let l = unsafe { &mut *self.logger };
        match event {
            CdcEvent::SessionStart => {
                l.connected.store(true, Ordering::Relaxed);
            }
            CdcEvent::SessionEnd => {
                // ignore
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
//  UsbLogger
// ---------------------------------------------------------------------------

/// Simple logger that writes to a dedicated CDC instance.
pub struct UsbLogger {
    cdc: CdcAcm,
    connected: AtomicBool,
}

static mut LOGGER_CDC: core::mem::MaybeUninit<UsbLogger> = core::mem::MaybeUninit::uninit();
static LOGGER_CDC_INITIALIZED: AtomicBool = AtomicBool::new(false);

unsafe impl Sync for UsbLogger {}

impl UsbLogger {
    /// Initialise the logger CDC on `instance`.
    pub fn init(instance: u32) {
        if LOGGER_CDC_INITIALIZED.load(Ordering::Relaxed) {
            return;
        }

        // Write uninitialised placeholder into static first
        unsafe {
            LOGGER_CDC.write(UsbLogger {
                cdc: core::mem::zeroed(),
                connected: AtomicBool::new(false),
            });
        }

        // Now use pointer to the static location (stable, never moves)
        let logger_ptr: *mut UsbLogger = unsafe { LOGGER_CDC.as_mut_ptr() };
        let handler = Box::new(LoggerCdcHandler { logger: logger_ptr });
        unsafe { &mut *logger_ptr }.cdc = CdcAcm::new(instance, handler);

        LOGGER_CDC_INITIALIZED.store(true, Ordering::Relaxed);
    }

    fn instance() -> &'static mut UsbLogger {
        unsafe { &mut *LOGGER_CDC.as_mut_ptr() }
    }

    /// Write data to the logger CDC.
    pub fn write(data: &[u8]) {
        let this = Self::instance();
        let n = this.cdc.write(data);
        if n > 0 {
            this.cdc.flush();
        }
    }

    /// Returns the number of bytes available for writing.
    pub fn write_available() -> u32 {
        // The CdcAcm doesn't expose writeAvailable directly,
        // but we can approximate by trying to write a small amount.
        // For now, return a reasonable default.
        64
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
pub extern "C" fn rn_usb_cdc_logger(n: i32, data: *const u8) {
    if n <= 0 || !LOGGER_CDC_INITIALIZED.load(Ordering::Relaxed) {
        return;
    }
    let slice = unsafe { core::slice::from_raw_parts(data, n as usize) };
    UsbLogger::write(slice);
}

/// Returns write space available on the logger CDC.
#[unsafe(no_mangle)]
pub extern "C" fn rn_usb_cdc_write_available() -> u32 {
    if !LOGGER_CDC_INITIALIZED.load(Ordering::Relaxed) {
        return 0;
    }
    UsbLogger::write_available()
}

// EOF
