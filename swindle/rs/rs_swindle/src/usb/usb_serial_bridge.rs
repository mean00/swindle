//! Rust-owned USB↔UART serial bridge.
//!
//! Replaces the C++ `BMPSerial` class in `bmp_serial.cpp`.
//! Bridges data between a CDC‑ACM (virtual COM port) and a hardware UART.

use alloc::boxed::Box;
use core::sync::atomic::{AtomicBool, Ordering};

use rust_esprit::{Cdc, CdcEvent, CdcEventHandler};
use rust_esprit::EventGroup;
use rust_esprit::{Serial, SerialEvent, SerialEventHandler};
use rust_esprit::delay_ms;

// ---------------------------------------------------------------------------
//  Constants & Strongly-Typed Event Mask
// ---------------------------------------------------------------------------

const BRIDGE_BUFFER_SIZE: usize = 256;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BridgeEvent(pub u32);

impl BridgeEvent {
    #[allow(dead_code)]
    pub const NONE: Self = Self(0);
    pub const UART_RX: Self = Self(1 << 0);
    pub const UART_TX_DONE: Self = Self(1 << 1);
    pub const USB_RX: Self = Self(1 << 2);
    pub const USB_TX_READY: Self = Self(1 << 3);
    pub const ALL: Self = Self(0x0f);

    #[inline(always)]
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

// ---------------------------------------------------------------------------
//  Bridge CDC event handler
// ---------------------------------------------------------------------------

/// Handler that implements `CdcEventHandler` for the bridge CDC.
struct BridgeCdcHandler;

impl CdcEventHandler for BridgeCdcHandler {
    fn handle(&self, _interface: i32, event: CdcEvent, payload: u32) {
        let b = UsbSerialBridge::instance();
        match event {
            CdcEvent::DataAvailable => {
                b.event_group.set_events(BridgeEvent::USB_RX.0);
            }
            CdcEvent::WriteAvailable => {
                b.event_group.set_events(BridgeEvent::USB_TX_READY.0);
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

static BRIDGE_CDC_HANDLER: BridgeCdcHandler = BridgeCdcHandler;

// ---------------------------------------------------------------------------
//  Bridge serial event handler
// ---------------------------------------------------------------------------

struct BridgeSerialHandler;

impl SerialEventHandler for BridgeSerialHandler {
    fn handle(&self, event: SerialEvent) {
        let b = UsbSerialBridge::instance();
        match event {
            SerialEvent::DataAvailable => {
                b.event_group.set_events(BridgeEvent::UART_RX.0);
            }
            SerialEvent::TxDone => {
                b.event_group.set_events(BridgeEvent::UART_TX_DONE.0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
//  Directional State Machine
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum DirectionState {
    /// Idle: waiting for input data from source.
    Idle,
    /// Transmitting: buffer holds data [dex..limit], sending to destination.
    Transmitting,
    /// Blocked: destination cannot accept more data; waiting for ready event.
    Blocked,
}

struct DirectionChannel {
    state: DirectionState,
    dex: usize,
    limit: usize,
    buffer: [u8; BRIDGE_BUFFER_SIZE],
}

impl DirectionChannel {
    const fn new() -> Self {
        DirectionChannel {
            state: DirectionState::Idle,
            dex: 0,
            limit: 0,
            buffer: [0u8; BRIDGE_BUFFER_SIZE],
        }
    }

    #[inline(always)]
    fn reset(&mut self) {
        self.state = DirectionState::Idle;
        self.dex = 0;
        self.limit = 0;
    }

    #[inline(always)]
    fn remaining(&self) -> usize {
        self.limit.saturating_sub(self.dex)
    }

    #[inline(always)]
    fn slice(&self) -> &[u8] {
        &self.buffer[self.dex..self.limit]
    }
}

// ---------------------------------------------------------------------------
//  UsbSerialBridge
// ---------------------------------------------------------------------------

/// Bridges a CDC‑ACM virtual serial port to a hardware UART.
///
/// Lives in a `static mut` (pinned for life) so the handler cookies stay valid.
pub struct UsbSerialBridge {
    cdc: Cdc,
    serial: Serial,
    event_group: EventGroup,
    connected: AtomicBool,
    usb2serial: DirectionChannel,
    serial2usb: DirectionChannel,
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
                serial: Serial::new(serial_instance, BRIDGE_BUFFER_SIZE as u32, true),
                event_group: EventGroup::new(),
                connected: AtomicBool::new(false),
                usb2serial: DirectionChannel::new(),
                serial2usb: DirectionChannel::new(),
            });
        }

        let bridge = Self::instance();

        // Attach serial event handler
        let serial_handler = Box::new(BridgeSerialHandler);
        bridge.serial.set_event_handler(serial_handler);

        // Init serial
        bridge.serial.init();
        bridge.serial.set_speed(baud);
        bridge.serial.enable_rx(false);

        // Create CDC with static event handler
        bridge.cdc = Cdc::new_static(usb_instance, &BRIDGE_CDC_HANDLER);

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
            let ev = BridgeEvent(this.event_group.wait_events(BridgeEvent::ALL.0, 20));
            Self::step_usb_to_serial(ev);
            Self::step_serial_to_usb(ev);
        }
    }

    /// Pure reactive state machine step: USB CDC -> Hardware UART.
    fn step_usb_to_serial(ev: BridgeEvent) {
        let this = Self::instance();

        // If Blocked, unblock upon UART TX completion
        if this.usb2serial.state == DirectionState::Blocked && ev.contains(BridgeEvent::UART_TX_DONE) {
            this.usb2serial.state = DirectionState::Transmitting;
        }

        // State Machine Loop (progresses immediately until waiting for an event)
        loop {
            match this.usb2serial.state {
                DirectionState::Idle => {
                    // Try to read next batch from USB if USB_RX signaled
                    if !ev.contains(BridgeEvent::USB_RX) {
                        break;
                    }
                    let nb = this.cdc.read(&mut this.usb2serial.buffer);
                    if nb <= 0 {
                        break;
                    }
                    this.usb2serial.dex = 0;
                    this.usb2serial.limit = nb as usize;
                    this.usb2serial.state = DirectionState::Transmitting;
                }
                DirectionState::Transmitting => {
                    if this.usb2serial.remaining() == 0 {
                        this.usb2serial.reset();
                        continue;
                    }
                    let sent = this.serial.transmit_no_block(this.usb2serial.slice());
                    if sent <= 0 {
                        // UART transmit buffer full; wait for UART_TX_DONE event
                        this.usb2serial.state = DirectionState::Blocked;
                        break;
                    }
                    this.usb2serial.dex += sent as usize;
                    if this.usb2serial.remaining() == 0 {
                        this.usb2serial.reset();
                        continue;
                    }
                }
                DirectionState::Blocked => {
                    // Waiting for UART_TX_DONE event
                    break;
                }
            }
        }
    }

    /// Pure reactive state machine step: Hardware UART -> USB CDC.
    fn step_serial_to_usb(ev: BridgeEvent) {
        let this = Self::instance();

        if !this.connected.load(Ordering::Relaxed) {
            this.serial.purge_rx();
            this.serial2usb.reset();
            return;
        }

        // If Blocked, unblock upon USB TX ready
        if this.serial2usb.state == DirectionState::Blocked && ev.contains(BridgeEvent::USB_TX_READY) {
            this.serial2usb.state = DirectionState::Transmitting;
        }

        // State Machine Loop
        loop {
            match this.serial2usb.state {
                DirectionState::Idle => {
                    // Try to read from UART ring buffer if UART_RX signaled
                    if !ev.contains(BridgeEvent::UART_RX) {
                        break;
                    }
                    let (nb, ptr) = this.serial.get_read_pointer();
                    if nb <= 0 {
                        break;
                    }
                    let count = core::cmp::min(nb as usize, BRIDGE_BUFFER_SIZE);
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            ptr,
                            this.serial2usb.buffer.as_mut_ptr(),
                            count,
                        );
                    }
                    this.serial.consume(count as u32);
                    this.serial2usb.dex = 0;
                    this.serial2usb.limit = count;
                    this.serial2usb.state = DirectionState::Transmitting;
                }
                DirectionState::Transmitting => {
                    if this.serial2usb.remaining() == 0 {
                        this.serial2usb.reset();
                        continue;
                    }
                    let written = this.cdc.write(this.serial2usb.slice());
                    if written <= 0 {
                        // USB CDC output buffer full; wait for USB_TX_READY
                        this.serial2usb.state = DirectionState::Blocked;
                        break;
                    }
                    this.cdc.flush();
                    this.serial2usb.dex += written as usize;
                    if this.serial2usb.remaining() == 0 {
                        this.serial2usb.reset();
                        continue;
                    }
                }
                DirectionState::Blocked => {
                    // Waiting for USB_TX_READY event
                    break;
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
