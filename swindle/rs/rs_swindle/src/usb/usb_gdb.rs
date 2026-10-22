//! Rust-owned GDB CDC (USB virtual serial port) handler.
//!
//! This module replaces the C++ `BufferGdb` class in `bmp_usb.cpp`.
//! It owns the `Cdc` instance for the GDB interface, handles
//! session events, and provides read/write primitives for the
//! GDB stub.

use core::sync::atomic::{AtomicBool, Ordering};

use rust_esprit::{Cdc, CdcEvent, CdcEventHandler};
use rust_esprit::EventGroup;
use rust_esprit::delay_ms;

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
const GDB_BUFFER_SIZE: usize = 1024;

// ---------------------------------------------------------------------------
//  GDB CDC event handler
// ---------------------------------------------------------------------------

/// Handler that implements `CdcEventHandler` for the C++ USB stack.
struct GdbCdcHandler;

impl CdcEventHandler for GdbCdcHandler {
    fn handle(&self, _interface: i32, event: CdcEvent, _payload: u32) {
        let gdb = GdbCdc::instance();
        match event {
            CdcEvent::DataAvailable => {
                gdb.event_group.set_events(GdbEvent::DATA_AVAILABLE.0);
            }
            CdcEvent::WriteAvailable => {
                // Ignored for GDB CDC
            }
            CdcEvent::SessionStart => {
                unsafe {
                    bmp_io_begin_session();
                }
                gdb.in_session.store(true, Ordering::Relaxed);
                gdb.event_group.set_events(GdbEvent::SESSION_START.0);
            }
            CdcEvent::SessionEnd => {
                unsafe {
                    bmp_io_end_session();
                }
                gdb.in_session.store(false, Ordering::Relaxed);
                gdb.event_group.set_events(GdbEvent::SESSION_END.0);
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

/// Safe wrapper around the GDB CDC instance.
///
/// All fields are plain owned values.  Lives in a `static mut` (pinned for
/// life) so the `GdbCdcHandler` cookie's raw pointer stays valid.
pub struct GdbCdc {
    cdc: Cdc,
    event_group: EventGroup,
    in_session: AtomicBool,
    buffer: [u8; GDB_BUFFER_SIZE],
}

// The GdbCdc lives here — pinned for the lifetime of the USB stack.
static mut GDB_CDC: core::mem::MaybeUninit<GdbCdc> = core::mem::MaybeUninit::uninit();
static GDB_CDC_INITIALIZED: AtomicBool = AtomicBool::new(false);

// GdbCdc is safe to access from a single task (FreeRTOS task context).
unsafe impl Sync for GdbCdc {}

impl GdbCdc {
    /// Create and initialise the GDB CDC instance.
    ///
    /// Must be called once before any other function.
    pub fn init(instance: u32) {
        if GDB_CDC_INITIALIZED.load(Ordering::Relaxed) {
            return;
        }

        // Write placeholder into static first (avoids Drop running on stack copy)
        unsafe {
            GDB_CDC.write(GdbCdc {
                cdc: core::mem::zeroed(),
                event_group: EventGroup::uninit(),
                in_session: AtomicBool::new(false),
                buffer: [0u8; GDB_BUFFER_SIZE],
            });
        }

        let this = Self::instance();
        // The event group is a `static` we own: create its kernel object now
        // (C-side `.bss` pool, no Arc, no heap).
        this.event_group.init_static();
        this.cdc = Cdc::new_static(instance, &GDB_HANDLER);

        GDB_CDC_INITIALIZED.store(true, Ordering::Relaxed);
    }

    /// Get a mutable reference to the global GDB CDC instance.
    fn instance() -> &'static mut GdbCdc {
        unsafe { &mut *GDB_CDC.as_mut_ptr() }
    }

    /// Take ownership of the event group (must be called from the task that owns it).
    pub fn take_ownership() {
        let this = Self::instance();
        this.event_group.take_ownership();
    }

    /// Wait for CDC events with a timeout.
    pub fn wait_events() -> GdbEvent {
        let this = Self::instance();
        GdbEvent(
            this.event_group
                .wait_events(0xffff, GDB_MAX_POLLING_PERIOD_MS as i32),
        )
    }

    /// Check if a GDB session is active.
    pub fn in_session() -> bool {
        let this = Self::instance();
        this.in_session.load(Ordering::Relaxed)
    }

    /// Read data from the CDC (non-blocking).
    pub fn read(buffer: &mut [u8]) -> i32 {
        let this = Self::instance();
        this.cdc.read(buffer)
    }

    /// Write data to the CDC (blocking).
    pub fn write(data: &[u8]) {
        let this = Self::instance();
        let mut remaining = data;
        while !remaining.is_empty() {
            let n = this.cdc.write(remaining);
            if n <= 0 {
                // No space available, yield briefly
                delay_ms(5);
                continue;
            }
            remaining = &remaining[n as usize..];
        }
    }

    /// Flush pending writes.
    pub fn flush() {
        let this = Self::instance();
        this.cdc.flush();
    }

    /// Read all pending CDC data and feed it to the GDB stub.
    /// Returns the number of packets processed.
    pub fn process_pending_data() -> i32 {
        let this = Self::instance();
        let mut count = 0;
        loop {
            let n = Self::read(&mut this.buffer);
            if n <= 0 {
                break;
            }
            // Feed the data to the GDB stub
            unsafe {
                rngdbstub_run(n as usize, this.buffer.as_ptr());
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
                rtt::swindle_reinit_rtt();
                unsafe {
                    rngdbstub_init();
                }
                connected = true;
            }
            if ev.contains(GdbEvent::SESSION_END) {
                unsafe {
                    rngdbstub_shutdown();
                }
                connected = false;
            }
            if ev.contains(GdbEvent::DATA_AVAILABLE) {
                GdbCdc::process_pending_data();
            }
        }
        if !connected {
            delay_ms(20);
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
