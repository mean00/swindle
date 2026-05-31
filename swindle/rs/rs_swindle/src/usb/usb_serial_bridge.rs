//! Rust-owned USB↔UART serial bridge.
//!
//! Replaces the C++ `BMPSerial` class in `bmp_serial.cpp`.
//! Bridges data between a CDC‑ACM (virtual COM port) and a hardware UART.

use alloc::boxed::Box;
use core::sync::atomic::{AtomicBool, Ordering};

use rust_esprit::cdc::{CdcAcm, CdcEvent, CdcEventHandler};
use rust_esprit::event::EventGroup;
use rust_esprit::serial::{SerialEvent, SerialEventHandler, SerialRxTx};
use rust_esprit::task::delay_ms;

// ---------------------------------------------------------------------------
//  Constants
// ---------------------------------------------------------------------------

const BRIDGE_BUFFER_SIZE: usize = 256;

const SERIAL_EVENT: u32 = 1 << 0;
const USB_EVENT_READ: u32 = 1 << 1;
const USB_EVENT_WRITE: u32 = 1 << 2;
const USB_EVENTS: u32 = USB_EVENT_READ | USB_EVENT_WRITE;

// ---------------------------------------------------------------------------
//  Bridge CDC event handler
// ---------------------------------------------------------------------------

/// Thin handler that implements `CdcEventHandler` for the bridge CDC.
struct BridgeCdcHandler {
    bridge: *mut UsbSerialBridge,
}

impl CdcEventHandler for BridgeCdcHandler {
    fn handle(&self, _interface: i32, event: CdcEvent, payload: u32) {
        let b = unsafe { &mut *self.bridge };
        match event {
            CdcEvent::DataAvailable => {
                b.event_group.set_events(USB_EVENT_READ);
            }
            CdcEvent::WriteAvailable => {
                b.event_group.set_events(USB_EVENT_WRITE);
            }
            CdcEvent::SessionStart => {
                if !b.connected.load(Ordering::Relaxed) {
                    b.serial.enable_rx(true);
                }
                b.connected.store(true, Ordering::Relaxed);
            }
            CdcEvent::SessionEnd => {
                // DTR acts funky, so we don't clear connected
            }
            CdcEvent::SetSpeed => {
                b.serial.set_speed(payload);
            }
        }
    }
}

// ---------------------------------------------------------------------------
//  Bridge serial event handler
// ---------------------------------------------------------------------------

struct BridgeSerialHandler {
    bridge: *mut UsbSerialBridge,
}

impl SerialEventHandler for BridgeSerialHandler {
    fn handle(&self, event: SerialEvent) {
        let b = unsafe { &mut *self.bridge };
        match event {
            SerialEvent::DataAvailable | SerialEvent::TxDone => {
                b.event_group.set_events(SERIAL_EVENT);
            }
        }
    }
}

// ---------------------------------------------------------------------------
//  Pump state machine helper
// ---------------------------------------------------------------------------

struct Pump {
    txing: bool,
    dex: usize,
    limit: usize,
    buffer: [u8; BRIDGE_BUFFER_SIZE],
}

impl Pump {
    const fn new() -> Self {
        Pump {
            txing: false,
            dex: 0,
            limit: 0,
            buffer: [0u8; BRIDGE_BUFFER_SIZE],
        }
    }
}

// ---------------------------------------------------------------------------
//  UsbSerialBridge
// ---------------------------------------------------------------------------

/// Bridges a CDC‑ACM virtual serial port to a hardware UART.
///
/// Lives in a `static mut` (pinned for life) so the handler cookies stay valid.
pub struct UsbSerialBridge {
    cdc: CdcAcm,
    serial: SerialRxTx,
    event_group: EventGroup,
    connected: AtomicBool,
    usb2serial: Pump,
    serial2usb: Pump,
}

static mut BRIDGE: core::mem::MaybeUninit<UsbSerialBridge> = core::mem::MaybeUninit::uninit();
static BRIDGE_INITIALIZED: AtomicBool = AtomicBool::new(false);

unsafe impl Sync for UsbSerialBridge {}

impl UsbSerialBridge {
    /// Create and initialise the USB↔UART bridge.
    ///
    /// Must be called once before any other function.
    pub fn init(usb_instance: u32, serial_instance: u32, baud: u32) {
        if BRIDGE_INITIALIZED.load(Ordering::Relaxed) {
            return;
        }

        // Write uninitialised placeholder into static first
        unsafe {
            BRIDGE.write(UsbSerialBridge {
                cdc: core::mem::zeroed(),
                serial: SerialRxTx::new(serial_instance, BRIDGE_BUFFER_SIZE as u32, true),
                event_group: EventGroup::new(),
                connected: AtomicBool::new(false),
                usb2serial: Pump::new(),
                serial2usb: Pump::new(),
            });
        }

        // Use pointer to static location (stable, never moves)
        let bridge_ptr: *mut UsbSerialBridge = unsafe { BRIDGE.as_mut_ptr() };
        let bridge = unsafe { &mut *bridge_ptr };

        // Attach serial event handler
        let serial_handler = Box::new(BridgeSerialHandler { bridge: bridge_ptr });
        bridge.serial.set_event_handler(serial_handler);

        // Init serial
        bridge.serial.init();
        bridge.serial.set_speed(baud);
        bridge.serial.enable_rx(false);

        // Create CDC with event handler
        let cdc_handler = Box::new(BridgeCdcHandler { bridge: bridge_ptr });
        bridge.cdc = CdcAcm::new(usb_instance, cdc_handler);

        BRIDGE_INITIALIZED.store(true, Ordering::Relaxed);
    }

    fn instance() -> &'static mut UsbSerialBridge {
        unsafe { &mut *BRIDGE.as_mut_ptr() }
    }

    /// Take ownership of the event group (must be called from the owning task).
    pub fn take_ownership() {
        Self::instance().event_group.take_ownership();
    }

    /// Run the bridge main loop (never returns).
    pub fn run() -> ! {
        let this = Self::instance();
        delay_ms(50); // let the GDB part start first

        loop {
            let ev = this.event_group.wait_events(SERIAL_EVENT | USB_EVENTS, 20);
            Self::run_usb_to_serial(ev);
            Self::run_serial_to_usb(ev);
        }
    }

    /// Pump data from USB CDC to UART serial.
    fn run_usb_to_serial(ev: u32) -> bool {
        let this = Self::instance();
        loop {
            match this.usb2serial.txing {
                false => {
                    // Idle: try to read from USB
                    if (ev & USB_EVENT_READ) == 0 {
                        return false;
                    }
                    let nb = this.cdc.read(&mut this.usb2serial.buffer);
                    if nb <= 0 {
                        return false;
                    }
                    this.usb2serial.txing = true;
                    this.usb2serial.limit = nb as usize;
                    this.usb2serial.dex = 0;
                    // Fall through to try sending immediately
                }
                true => {
                    // Sending: pump data to serial
                    let avail = this.usb2serial.limit - this.usb2serial.dex;
                    if avail == 0 {
                        this.usb2serial.txing = false;
                        return true;
                    }
                    let ptr = &this.usb2serial.buffer[this.usb2serial.dex..this.usb2serial.limit];
                    let txed = this.serial.transmit_no_block(ptr);
                    if txed <= 0 {
                        return false; // wait for tx done event
                    }
                    this.usb2serial.dex += txed as usize;
                    if this.usb2serial.dex >= this.usb2serial.limit {
                        this.usb2serial.txing = false;
                        // Re-evaluate at next loop
                        this.event_group.set_events(USB_EVENTS);
                    }
                    return true;
                }
            }
        }
    }

    /// Pump data from UART serial to USB CDC.
    fn run_serial_to_usb(ev: u32) -> bool {
        let this = Self::instance();

        // If not connected, just purge serial rx
        if !this.connected.load(Ordering::Relaxed) {
            this.serial.purge_rx();
            return false;
        }

        // loop
        {
            match this.serial2usb.txing {
                false => {
                    // Idle: try to read from serial
                    if (ev & SERIAL_EVENT) == 0 {
                        return false;
                    }
                    let (nb, ptr) = this.serial.get_read_pointer();
                    if nb <= 0 {
                        return false;
                    }
                    let nb = core::cmp::min(nb as usize, BRIDGE_BUFFER_SIZE);
                    // Copy from serial ring buffer
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            ptr,
                            this.serial2usb.buffer.as_mut_ptr(),
                            nb,
                        );
                    }
                    this.serial.consume(nb as u32);
                    this.serial2usb.txing = true;
                    this.serial2usb.dex = 0;
                    this.serial2usb.limit = nb;
                    true
                }
                true => {
                    // Sending: pump data to USB
                    if !this.connected.load(Ordering::Relaxed) {
                        this.serial2usb.txing = false;
                        return false;
                    }
                    let avail = this.serial2usb.limit - this.serial2usb.dex;
                    if avail == 0 {
                        this.serial2usb.txing = false;
                        return true;
                    }
                    let ptr = &this.serial2usb.buffer[this.serial2usb.dex..this.serial2usb.limit];
                    let consumed = this.cdc.write(ptr);
                    if consumed <= 0 {
                        return false; // wait for write available
                    }
                    this.cdc.flush();
                    this.serial2usb.dex += consumed as usize;
                    if this.serial2usb.dex >= this.serial2usb.limit {
                        this.serial2usb.txing = false;
                        this.event_group.set_events(SERIAL_EVENT);
                    }
                    true
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
//  C-compatible exports for the C++ init code
// ---------------------------------------------------------------------------

/// Initialise the USB↔UART bridge (called from C++ `serialInit` equivalent).
#[unsafe(no_mangle)]
pub extern "C" fn rn_serial_bridge_init(usb_instance: u32, serial_instance: u32, baud: u32) {
    UsbSerialBridge::init(usb_instance, serial_instance, baud);
}

/// Run the bridge task (called from a dedicated FreeRTOS task).
#[unsafe(no_mangle)]
pub extern "C" fn rn_serial_bridge_task() {
    UsbSerialBridge::take_ownership();
    UsbSerialBridge::run();
}

/// Write data to the bridge CDC (used for log/RTT redirect on 2-CDC builds).
#[unsafe(no_mangle)]
pub extern "C" fn rn_serial_bridge_write(n: u32, data: *const u8) {
    if n == 0 || !BRIDGE_INITIALIZED.load(Ordering::Relaxed) {
        return;
    }
    let slice = unsafe { core::slice::from_raw_parts(data, n as usize) };
    let this = unsafe { &mut *BRIDGE.as_mut_ptr() };
    let written = this.cdc.write(slice);
    if written > 0 {
        this.cdc.flush();
    }
}

// EOF
