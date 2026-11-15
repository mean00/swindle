//! Rust-owned GDB CDC (USB virtual serial port) handler.
//!
//! This module replaces the C++ `BufferGdb` class in `bmp_usb.cpp`.
//! It owns the `Cdc` instance for the GDB interface, handles
//! session events, and provides read/write primitives for the
//! GDB stub.

use core::sync::atomic::{AtomicBool, Ordering};

use rust_esprit::EventGroup;
use rust_esprit::delay_ms;
use rust_esprit::{Cdc, CdcEvent, CdcEventHandler};

use crate::SyncCell;
use crate::bmp;
use crate::rtt;

// ---------------------------------------------------------------------------
//  Constants & Typed Events
// ---------------------------------------------------------------------------

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GdbEvent(pub u32);

impl GdbEvent {
    pub const NONE: Self = Self(0);
    pub const DATA_AVAILABLE: Self = Self(1 << 0);
    pub const SESSION_START: Self = Self(1 << 1);
    pub const SESSION_END: Self = Self(1 << 2);

    #[inline(always)]
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

const GDB_MAX_POLLING_PERIOD_MS: u32 = 20;
const GDB_IDLE_POLL_DELAY_MS: u32 = 20;
const GDB_ALL_EVENTS: u32 = 0xffff;
const GDB_WRITE_TIMEOUT_MS: u32 = 1000;
const GDB_BUFFER_SIZE: usize = 1024;

// ---------------------------------------------------------------------------
//  GDB CDC event handler
// ---------------------------------------------------------------------------

/// Handler that implements `CdcEventHandler` for the GDB CDC.
///
/// Runs in the `usbd` task context: only updates atomic state and signals the
/// event group; GPIO reconfiguration (`bmp_io_begin_session` / `bmp_io_end_session`)
/// is deferred to `rngdb_gdb_task` so it cannot race with `rngdbstub_poll()`.
struct GdbCdcHandler;

impl CdcEventHandler for GdbCdcHandler {
    fn handle(&self, _interface: i32, event: CdcEvent, _payload: u32) {
        match event {
            CdcEvent::DataAvailable => {
                SHARED.signal(GdbEvent::DATA_AVAILABLE);
            }
            CdcEvent::WriteAvailable => {
                // Ignored for GDB CDC
            }
            CdcEvent::SessionStart => {
                SHARED.in_session.store(true, Ordering::Relaxed);
                SHARED.signal(GdbEvent::SESSION_START);
            }
            CdcEvent::SessionEnd => {
                SHARED.in_session.store(false, Ordering::Relaxed);
                SHARED.signal(GdbEvent::SESSION_END);
            }
            CdcEvent::SetSpeed => {
                // Ignored for GDB CDC
            }
        }
    }
}

static GDB_HANDLER: GdbCdcHandler = GdbCdcHandler;

// ---------------------------------------------------------------------------
//  GDB CDC public handle
// ---------------------------------------------------------------------------

struct GdbShared {
    event_group: SyncCell<EventGroup>,
    in_session: AtomicBool,
    cdc: SyncCell<core::mem::MaybeUninit<Cdc>>,
    buffer: SyncCell<[u8; GDB_BUFFER_SIZE]>,
}

// SAFETY: `in_session` is atomic, `event_group` delegates to thread-safe FreeRTOS
// primitives, `cdc` is initialized once before use and delegates to the C CDC driver,
// and `buffer` is only used by the single GDB task in `process_pending_data`.
unsafe impl Sync for GdbShared {}

impl GdbShared {
    #[inline(always)]
    fn signal(&self, ev: GdbEvent) {
        unsafe { (*self.event_group.as_ptr()).set_events(ev.0) };
    }

    #[inline(always)]
    fn cdc(&self) -> &Cdc {
        // SAFETY: Initialized in `GdbCdc::init` before any read/write/flush call.
        unsafe { (*self.cdc.as_ptr()).assume_init_ref() }
    }
}

static SHARED: GdbShared = GdbShared {
    event_group: SyncCell::new(EventGroup::uninit()),
    in_session: AtomicBool::new(false),
    cdc: SyncCell::new(core::mem::MaybeUninit::uninit()),
    buffer: SyncCell::new([0u8; GDB_BUFFER_SIZE]),
};
static GDB_CDC_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Safe wrapper around the GDB CDC instance.
pub struct GdbCdc;

impl GdbCdc {
    /// Create and initialise the GDB CDC instance.
    ///
    /// Must be called once before any other function.
    pub fn init(instance: u32) {
        if GDB_CDC_INITIALIZED.load(Ordering::Acquire) {
            return;
        }

        // The event group is a `static` we own: create its kernel object now
        // (C-side `.bss` pool, no Arc, no heap).
        unsafe {
            (*SHARED.event_group.as_ptr()).init_static();
            (*SHARED.cdc.as_ptr()).write(Cdc::new_static(instance, &GDB_HANDLER));
        }

        GDB_CDC_INITIALIZED.store(true, Ordering::Release);
    }

    /// Take ownership of the event group (must be called from the task that owns it).
    pub fn take_ownership() {
        unsafe { (*SHARED.event_group.as_ptr()).take_ownership() };
    }

    /// Wait for CDC events with a timeout.
    pub fn wait_events() -> GdbEvent {
        let ev = unsafe {
            (*SHARED.event_group.as_ptr())
                .wait_events(GDB_ALL_EVENTS, GDB_MAX_POLLING_PERIOD_MS as i32)
        };
        GdbEvent(ev)
    }

    /// Check if a GDB session is active.
    pub fn in_session() -> bool {
        SHARED.in_session.load(Ordering::Relaxed)
    }

    /// Read data from the CDC (non-blocking).
    pub fn read(buffer: &mut [u8]) -> usize {
        SHARED.cdc().read(buffer)
    }

    /// Write data to the CDC (bounded so a detached host cannot hang `gdb_task`).
    pub fn write(data: &[u8]) {
        let _ = SHARED.cdc().write_timeout(data, GDB_WRITE_TIMEOUT_MS);
    }

    /// Flush pending writes.
    pub fn flush() {
        SHARED.cdc().flush_once();
    }

    /// Read all pending CDC data and feed it to the GDB stub.
    /// Returns the number of packets processed.
    pub fn process_pending_data() -> i32 {
        let buf = unsafe { &mut *SHARED.buffer.as_ptr() };
        let mut count = 0;
        loop {
            let n = Self::read(buf);
            if n == 0 {
                break;
            }
            // Feed the data to the GDB stub
            unsafe {
                rngdbstub_run(n, buf.as_ptr());
            }
            count += 1;
        }
        count
    }
}

// ---------------------------------------------------------------------------
//  C-compatible exports for the GDB stub
// ---------------------------------------------------------------------------

/// Send data to the host via GDB CDC (called from Rust GDB stub).
/// Not used in network builds (C++ provides its own implementation).
#[cfg(not(feature = "network"))]
#[unsafe(no_mangle)]
pub extern "C" fn rngdb_send_data_c(sz: u32, ptr: *const u8) {
    if !GdbCdc::in_session() {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(ptr, sz as usize) };
    GdbCdc::write(data);
}

/// Flush GDB CDC output (called from Rust GDB stub).
/// Not used in network builds (C++ provides its own implementation).
#[cfg(not(feature = "network"))]
#[unsafe(no_mangle)]
pub extern "C" fn rngdb_output_flush_c() {
    if GdbCdc::in_session() {
        GdbCdc::flush();
    }
}

// ---------------------------------------------------------------------------
//  C-compatible exports for the C++ init code
// ---------------------------------------------------------------------------

/// Initialise the GDB CDC (called from C++ `gdb_if_init`).
#[unsafe(no_mangle)]
pub extern "C" fn rngdb_cdc_init(instance: u32) {
    GdbCdc::init(instance);
}

/// Main GDB task loop (called from C++ `gdb_task`).
/// Replaces the old C++ while(true) loop that called individual rngdb_cdc_* functions.
#[unsafe(no_mangle)]
pub extern "C" fn rngdb_gdb_task() {
    GdbCdc::take_ownership();
    let mut connected = false;
    loop {
        let ev = GdbCdc::wait_events();
        if connected {
            // Check if the target reached a breakpoint/watchpoint/...
            unsafe {
                rngdbstub_poll();
            }
            if bmp::bmp_attached() {
                if rtt::swindle_rtt_enabled() {
                    rtt::swindle_run_rtt();
                } else {
                    rtt::swindle_purge_rtt();
                }
            }
        }
        if ev != GdbEvent::NONE {
            if ev.contains(GdbEvent::SESSION_START) {
                unsafe {
                    bmp_io_begin_session();
                }
                rtt::swindle_reinit_rtt();
                unsafe {
                    rngdbstub_init();
                }
                connected = true;
            }
            if ev.contains(GdbEvent::SESSION_END) {
                unsafe {
                    rngdbstub_shutdown();
                    bmp_io_end_session();
                }
                connected = false;
            }
            if ev.contains(GdbEvent::DATA_AVAILABLE) {
                GdbCdc::process_pending_data();
            }
        }
        if !connected {
            delay_ms(GDB_IDLE_POLL_DELAY_MS);
        }
    }
}

// ---------------------------------------------------------------------------
//  External C functions we need
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn bmp_io_begin_session();
    fn bmp_io_end_session();
    fn rngdbstub_run(len: usize, data: *const u8);
    fn rngdbstub_init();
    fn rngdbstub_shutdown();
    fn rngdbstub_poll();
}
