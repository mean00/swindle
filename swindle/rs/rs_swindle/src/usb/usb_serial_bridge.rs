//! Rust-owned USB↔UART serial bridge.
//!
//! Replaces the C++ `BMPSerial` class in `bmp_serial.cpp`.
//! Bridges data between a CDC‑ACM (virtual COM port) and a hardware UART.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::SyncCell;
use rust_esprit::EventGroup;
use rust_esprit::Mutex;
use rust_esprit::UartDriver;
use rust_esprit::delay_ms;
use rust_esprit::hal::Error as UartError;
use rust_esprit::hal::uart::{Uart as UartTrait, UartConfig, UartHandler};
use rust_esprit::{Cdc, CdcEvent, CdcEventHandler};

// ---------------------------------------------------------------------------
//  Constants & Strongly-Typed Event Mask
// ---------------------------------------------------------------------------

const BRIDGE_BUFFER_SIZE: usize = 256;

/// Scratch size used to drop stale UART bytes while the host is not connected.
const PURGE_CHUNK: usize = 32;

/// Initial delay before entering the bridge loop so the GDB task initializes first.
const BRIDGE_STARTUP_DELAY_MS: u32 = 50;

/// Event-group poll timeout in the bridge task loop.
const BRIDGE_EVENT_WAIT_MS: i32 = 20;

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
    /// `mon redirect` queued log bytes for the CDC - see [`LOG_RING`].
    pub const LOG: Self = Self(1 << 4);
    pub const ALL: Self = Self(0x1f);

    #[inline(always)]
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

// ---------------------------------------------------------------------------
//  The log path: a single-writer queue
// ---------------------------------------------------------------------------

/// Bytes the `mon redirect` sink hands to the bridge CDC.
///
/// The CDC has exactly **one writer** - the bridge task. The sink only queues here,
/// because writing straight to tinyUSB from the logger's context (which can be the
/// `tud_cdc_tx_complete_cb` chain when a host is reading) re-enters the FIFO the
/// bridge task is in and stops the console (see `usb.md` 9b).
const LOG_RING_SIZE: usize = 512;

struct LogRing {
    buf: core::cell::UnsafeCell<[u8; LOG_RING_SIZE]>,
    head: core::sync::atomic::AtomicUsize, // next byte to write (producer)
    tail: core::sync::atomic::AtomicUsize, // next byte to read (consumer)
}

// SAFETY: single producer (whoever logs) + single consumer (the bridge task), and the
// two indices are atomics ordered acquire/release.
unsafe impl Sync for LogRing {}

impl LogRing {
    const fn new() -> Self {
        Self {
            buf: core::cell::UnsafeCell::new([0u8; LOG_RING_SIZE]),
            head: core::sync::atomic::AtomicUsize::new(0),
            tail: core::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Queue as much of `data` as fits; returns how much was taken (the rest is dropped).
    fn push(&self, data: &[u8]) -> usize {
        use core::sync::atomic::Ordering;
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        let used = head.wrapping_sub(tail);
        let free = LOG_RING_SIZE - used;
        let n = data.len().min(free);
        for (i, b) in data[..n].iter().enumerate() {
            // SAFETY: the producer owns [head, head+n), n <= free.
            unsafe { *self.buf.get().cast::<u8>().add((head + i) % LOG_RING_SIZE) = *b };
        }
        self.head.store(head.wrapping_add(n), Ordering::Release);
        n
    }

    /// Peek up to `out.len()` bytes without advancing `tail`; the consumer is the bridge task only.
    fn peek(&self, out: &mut [u8]) -> usize {
        use core::sync::atomic::Ordering;
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        let used = head.wrapping_sub(tail);
        let n = out.len().min(used);
        for (i, b) in out[..n].iter_mut().enumerate() {
            // SAFETY: the consumer owns [tail, tail+n), n <= used.
            *b = unsafe { *self.buf.get().cast::<u8>().add((tail + i) % LOG_RING_SIZE) };
        }
        n
    }

    /// Advance `tail` by `count` bytes after they have been written to the CDC.
    fn consume(&self, count: usize) {
        use core::sync::atomic::Ordering;
        let tail = self.tail.load(Ordering::Relaxed);
        self.tail.store(tail.wrapping_add(count), Ordering::Release);
    }

    /// Drop all queued bytes; called only by the consumer when no host is connected.
    fn clear(&self) {
        use core::sync::atomic::Ordering;
        let head = self.head.load(Ordering::Acquire);
        self.tail.store(head, Ordering::Release);
    }
}

static LOG_RING: LogRing = LogRing::new();

// ---------------------------------------------------------------------------
//  Bridge Shared State & Handlers
// ---------------------------------------------------------------------------

struct BridgeShared {
    event_group: SyncCell<EventGroup>,
    connected: AtomicBool,
    session_start_requested: AtomicBool,
    pending_baud: AtomicU32,
    cdc: SyncCell<core::mem::MaybeUninit<Cdc>>,
    serial: SyncCell<Option<UartDriver>>,
}

// SAFETY: All fields are either inherently thread-safe (AtomicBool, AtomicU32)
// or interior-mutable containers where `serial` is written once at init and taken by
// the dedicated task, `event_group` uses thread-safe FreeRTOS primitives, and
// `cdc` delegates to the underlying C CDC driver which handles endpoint FIFOs.
unsafe impl Sync for BridgeShared {}

impl BridgeShared {
    #[inline(always)]
    fn signal(&self, ev: BridgeEvent) {
        unsafe { (*self.event_group.as_ptr()).set_events(ev.0) };
    }

    #[inline(always)]
    fn cdc(&self) -> &Cdc {
        // SAFETY: Initialized in `init` before any event handler or write call.
        unsafe { (*self.cdc.as_ptr()).assume_init_ref() }
    }
}

static SHARED: BridgeShared = BridgeShared {
    event_group: SyncCell::new(EventGroup::uninit()),
    connected: AtomicBool::new(false),
    session_start_requested: AtomicBool::new(false),
    pending_baud: AtomicU32::new(0),
    cdc: SyncCell::new(core::mem::MaybeUninit::uninit()),
    serial: SyncCell::new(None),
};

static BRIDGE_INITIALIZED: AtomicBool = AtomicBool::new(false);
/// Serializes TX FIFO writes between `usbserial` (`step_serial_to_usb`, `drain_log`)
/// and `gdb_task` (`rn_serial_bridge_write`).
static TX_MUTEX: Mutex<()> = Mutex::new(());

/// Handler that implements `CdcEventHandler` for the bridge CDC.
struct BridgeCdcHandler;

impl CdcEventHandler for BridgeCdcHandler {
    fn handle(&self, _interface: i32, event: CdcEvent, payload: u32) {
        match event {
            CdcEvent::DataAvailable => {
                SHARED.signal(BridgeEvent::USB_RX);
            }
            CdcEvent::WriteAvailable => {
                SHARED.signal(BridgeEvent::USB_TX_READY);
            }
            CdcEvent::SessionStart => {
                SHARED.connected.store(true, Ordering::Relaxed);
                SHARED
                    .session_start_requested
                    .store(true, Ordering::Relaxed);
                SHARED.signal(BridgeEvent::USB_RX);
            }
            CdcEvent::SessionEnd => {
                // DTR acts funky, so we don't clear connected
            }
            CdcEvent::SetSpeed => {
                SHARED.pending_baud.store(payload, Ordering::Relaxed);
                SHARED.signal(BridgeEvent::USB_RX);
            }
        }
    }
}

static BRIDGE_CDC_HANDLER: BridgeCdcHandler = BridgeCdcHandler;

// ---------------------------------------------------------------------------
//  Bridge serial (UART) event handler
// ---------------------------------------------------------------------------

/// The bridge's `UartHandler`: it only translates driver events into the
/// `BridgeEvent` bits the loop waits on.
///
/// The receive side is a ring in the driver, so `on_rx_available` is all that is
/// needed — the bytes are read with `Uart::read` when the loop wakes up. The
/// transmit side reports `on_tx_complete` when everything queued has left, which
/// is what releases a `Blocked` direction.
struct BridgeSerialHandler;

impl UartHandler for BridgeSerialHandler {
    fn on_rx_available(&self) {
        SHARED.signal(BridgeEvent::UART_RX);
    }

    fn on_tx_complete(&self) {
        SHARED.signal(BridgeEvent::UART_TX_DONE);
    }

    fn on_error(&self, _error: UartError) {
        // Overrun/framing errors are recoverable: the ring keeps what it has and
        // the next `read` resumes. Nothing to report upwards.
    }
}

/// The bridge's UART handler. It used to be `Box::new`ed at init and leaked by
/// `Serial::set_event_handler` (malloc_used.md C10); a zero-sized static lives
/// in `.rodata` and costs no RAM at all.
static BRIDGE_SERIAL_HANDLER: BridgeSerialHandler = BridgeSerialHandler;

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
//  UsbSerialBridge & Dedicated Task
// ---------------------------------------------------------------------------

pub struct UsbSerialBridge;

impl UsbSerialBridge {
    /// Create and initialise the USB↔UART bridge.
    ///
    /// Must be called once before any other function.
    pub fn init(usb_instance: u32, serial_instance: u32, baud: u32) {
        if BRIDGE_INITIALIZED.load(Ordering::Acquire) {
            return;
        }

        unsafe {
            (*SHARED.event_group.as_ptr()).init_static();

            // Open the UART the way a pass-through needs it: an interrupt-driven
            // receive ring (the handler reports `on_rx_available`) and a buffered
            // transmitter (`try_write` takes what fits). The handler is installed
            // before the first byte can arrive.
            let mut uart = UartDriver::with_dma(
                serial_instance,
                &UartConfig {
                    baudrate: baud,
                    ..UartConfig::default()
                },
            )
            .expect("UART instance of the bridge");
            uart.set_handler(Some(&BRIDGE_SERIAL_HANDLER));
            let _ = uart.enable_rx(false);
            *SHARED.serial.as_ptr() = Some(uart);

            // Create CDC with static event handler
            let cdc = Cdc::new_static(usb_instance, &BRIDGE_CDC_HANDLER);
            (*SHARED.cdc.as_ptr()).write(cdc);
        }

        BRIDGE_INITIALIZED.store(true, Ordering::Release);
    }

    /// Take ownership of the event group (must be called from the owning task).
    pub fn take_ownership() {
        unsafe {
            (*SHARED.event_group.as_ptr()).take_ownership();
        }
    }

    /// Run the bridge main loop (never returns).
    pub fn run() -> ! {
        let serial = unsafe { (*SHARED.serial.as_ptr()).take() };
        let mut task = BridgeTask {
            serial,
            usb2serial: DirectionChannel::new(),
            serial2usb: DirectionChannel::new(),
        };
        task.run();
    }
}

struct BridgeTask {
    serial: Option<UartDriver>,
    usb2serial: DirectionChannel,
    serial2usb: DirectionChannel,
}

impl BridgeTask {
    fn run(&mut self) -> ! {
        delay_ms(BRIDGE_STARTUP_DELAY_MS); // let the GDB part start first

        loop {
            let ev = BridgeEvent(unsafe {
                (*SHARED.event_group.as_ptr()).wait_events(BridgeEvent::ALL.0, BRIDGE_EVENT_WAIT_MS)
            });

            // Apply deferred CDC session requests in task context
            if SHARED.session_start_requested.load(Ordering::Relaxed) {
                SHARED
                    .session_start_requested
                    .store(false, Ordering::Relaxed);
                if let Some(uart) = self.serial.as_mut() {
                    let _ = uart.enable_rx(true);
                }
            }
            let baud = SHARED.pending_baud.load(Ordering::Relaxed);
            if baud != 0 {
                SHARED.pending_baud.store(0, Ordering::Relaxed);
                if let Some(uart) = self.serial.as_mut() {
                    let _ = uart.set_baudrate(baud);
                }
            }

            self.step_usb_to_serial(ev);
            self.step_serial_to_usb(ev);
            self.drain_log();
        }
    }

    /// Move queued log bytes to the CDC.
    ///
    /// Uses `LOG_RING.peek` + `consume` so the bridge task never calls `push()`
    /// (preserving single-producer / single-consumer on `LOG_RING`) and holds
    /// `TX_MUTEX` (non-blocking `try_lock`) so TX FIFO writes never race with
    /// `rn_serial_bridge_write`.
    fn drain_log(&mut self) {
        const CHUNK: usize = 128;
        if !SHARED.connected.load(Ordering::Relaxed) {
            // Nobody listening: drop, otherwise the queue fills for ever.
            LOG_RING.clear();
            return;
        }
        let Some(_guard) = TX_MUTEX.try_lock() else {
            return;
        };
        let mut chunk = [0u8; CHUNK];
        loop {
            let n = LOG_RING.peek(&mut chunk);
            if n == 0 {
                break;
            }
            let written = SHARED.cdc().write_no_block(&chunk[..n]);
            LOG_RING.consume(written);
            if written < n {
                // FIFO full: leave the unwritten tail in `LOG_RING` for the next
                // `USB_TX_READY` (tx complete) wakeup.
                break;
            }
        }
        SHARED.cdc().flush_once();
    }

    /// Pure reactive state machine step: USB CDC -> Hardware UART.
    fn step_usb_to_serial(&mut self, ev: BridgeEvent) {
        // If Blocked, unblock upon UART TX completion
        if self.usb2serial.state == DirectionState::Blocked
            && ev.contains(BridgeEvent::UART_TX_DONE)
        {
            self.usb2serial.state = DirectionState::Transmitting;
        }

        // State Machine Loop (progresses immediately until waiting for an event)
        loop {
            match self.usb2serial.state {
                DirectionState::Idle => {
                    // Try to read next batch from USB if USB_RX signaled
                    if !ev.contains(BridgeEvent::USB_RX) {
                        break;
                    }
                    let nb = SHARED.cdc().read(&mut self.usb2serial.buffer);
                    if nb == 0 {
                        break;
                    }
                    self.usb2serial.dex = 0;
                    self.usb2serial.limit = nb;
                    self.usb2serial.state = DirectionState::Transmitting;
                }
                DirectionState::Transmitting => {
                    if self.usb2serial.remaining() == 0 {
                        self.usb2serial.reset();
                        continue;
                    }
                    let Some(uart) = self.serial.as_mut() else {
                        self.usb2serial.state = DirectionState::Blocked;
                        break;
                    };
                    let sent = uart.try_write(self.usb2serial.slice()).unwrap_or(0);
                    if sent == 0 {
                        // UART transmit ring full; `on_tx_complete` unblocks us.
                        self.usb2serial.state = DirectionState::Blocked;
                        break;
                    }
                    self.usb2serial.dex += sent;
                    if self.usb2serial.remaining() == 0 {
                        self.usb2serial.reset();
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
    fn step_serial_to_usb(&mut self, ev: BridgeEvent) {
        if !SHARED.connected.load(Ordering::Relaxed) {
            // Drop whatever arrived while the host was not listening.
            if let Some(uart) = self.serial.as_mut() {
                let mut scratch = [0u8; PURGE_CHUNK];
                while uart.read(&mut scratch).unwrap_or(0) > 0 {}
            }
            self.serial2usb.reset();
            return;
        }

        // If Blocked, unblock upon USB TX ready
        if self.serial2usb.state == DirectionState::Blocked
            && ev.contains(BridgeEvent::USB_TX_READY)
        {
            self.serial2usb.state = DirectionState::Transmitting;
        }

        // State Machine Loop
        loop {
            match self.serial2usb.state {
                DirectionState::Idle => {
                    // Try to read from UART ring buffer if UART_RX signaled
                    if !ev.contains(BridgeEvent::UART_RX) {
                        break;
                    }
                    let Some(uart) = self.serial.as_mut() else {
                        break;
                    };
                    let count = uart.read(&mut self.serial2usb.buffer).unwrap_or(0);
                    if count == 0 {
                        break;
                    }
                    self.serial2usb.dex = 0;
                    self.serial2usb.limit = count;
                    self.serial2usb.state = DirectionState::Transmitting;
                }
                DirectionState::Transmitting => {
                    if self.serial2usb.remaining() == 0 {
                        self.serial2usb.reset();
                        continue;
                    }
                    let written = {
                        let _guard = TX_MUTEX.lock();
                        let w = SHARED.cdc().write_no_block(self.serial2usb.slice());
                        if w > 0 {
                            SHARED.cdc().flush_once();
                        }
                        w
                    };
                    if written == 0 {
                        // USB CDC output buffer full; wait for USB_TX_READY
                        self.serial2usb.state = DirectionState::Blocked;
                        break;
                    }
                    self.serial2usb.dex += written;
                    if self.serial2usb.remaining() == 0 {
                        self.serial2usb.reset();
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

/// Non-blocking write to the bridge CDC, for the `mon redirect` log sink.
///
/// `rn_serial_bridge_write` waits for the host to drain the CDC FIFO (the C++
/// `lnUsbCDC::write` did the same), which is fine for a data path but not for the
/// logger: with no reader attached the redirect would park the console for ever.
/// This one takes what fits and drops the rest, the way the UART logger does.
#[unsafe(no_mangle)]
pub extern "C" fn rn_serial_bridge_write_nb(n: u32, data: *const u8) {
    if n == 0 || !BRIDGE_INITIALIZED.load(Ordering::Acquire) {
        return;
    }
    let slice = unsafe { core::slice::from_raw_parts(data, n as usize) };
    if LOG_RING.push(slice) > 0 {
        // Wake the bridge task so `drain_log` can flush it.
        SHARED.signal(BridgeEvent::LOG);
    }
}

/// How long the bridge waits for the host to drain the CDC FIFO before giving up on
/// a chunk. A host that stops reading must not park the task for ever.
const BRIDGE_WRITE_TIMEOUT_MS: u32 = 250;

/// Write data to the bridge CDC.
///
/// Bounded on purpose: `Cdc::write` loops until the host drains the FIFO (the C++
/// `lnUsbCDC::write` did the same), so a host that detaches in the middle of a stream
/// used to wedge the caller. Here a chunk waits at most `BRIDGE_WRITE_TIMEOUT_MS` and
/// the remainder is dropped. Serialized with `TX_MUTEX` so RTT and UART-bridge bytes
/// never interleave in TinyUSB's TX FIFO.
#[unsafe(no_mangle)]
pub extern "C" fn rn_serial_bridge_write(n: u32, data: *const u8) {
    if n == 0 || !BRIDGE_INITIALIZED.load(Ordering::Acquire) {
        return;
    }
    let slice = unsafe { core::slice::from_raw_parts(data, n as usize) };
    let _guard = TX_MUTEX.lock();
    let _ = SHARED.cdc().write_timeout(slice, BRIDGE_WRITE_TIMEOUT_MS);
}

// EOF
