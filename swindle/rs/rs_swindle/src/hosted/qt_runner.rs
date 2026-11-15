//! Hosted GDB server: the Rust [`SocketRunner`] over the Qt socket layer.
//!
//! `hosted/src/esp_socket_rust.cpp` provides the bytes (`ln_socket_*`, the
//! `lnSocketQt` server) and the probe link (`probe_link_*`); everything else —
//! slot bits, session state, decode/dispatch, the relay — is the same runner the
//! W5500 probe build uses. The C++ `GdbServerThread` keeps the thread and the
//! poll tick: it calls the `hosted_runner_*` entry points below.

use alloc::boxed::Box;
use core::ffi::{c_char, c_void};

use rs_socketrunner::{
    EventGroup, Socket, SocketError, SocketEvent, SocketListener, SocketRunner, SocketRunnerHandler,
};

use crate::SyncCell;

/// Per-read buffer: GDB packets are far smaller, the relay copies what is there.
const RX_CHUNK: usize = 2048;
/// How long the relay waits for the probe's answer before returning to the loop.
const RELAY_REPLY_WAIT_MS: i32 = 50;
/// Replies staged while the BMP core runs (see [`rngdb_send_data_c`]).
const OUT_MAX: usize = 4096;

type SocketEventCb = extern "C" fn(event: u32, arg: *mut c_void);

unsafe extern "C" {
    // Qt side (hosted/src/esp_socket_rust.cpp).
    fn ln_socket_open(port: u16, cb: SocketEventCb, arg: *mut c_void) -> *mut c_void;
    fn ln_socket_read(sock: *mut c_void, dst: *mut u8, cap: u32, n: *mut u32) -> i32;
    fn ln_socket_write(sock: *mut c_void, n: u32, data: *const u8, done: *mut u32) -> i32;
    fn ln_socket_flush(sock: *mut c_void) -> i32;
    fn ln_socket_write_buffer_available(sock: *mut c_void, n: *mut u32) -> i32;
    fn ln_socket_disconnect_client(sock: *mut c_void) -> i32;
    fn ln_socket_close(sock: *mut c_void);

    fn probe_link_open(host: *const c_char, port: u16) -> *mut c_void;
    fn probe_link_is_open(link: *mut c_void) -> bool;
    fn probe_link_read(link: *mut c_void, wait_ms: i32, dst: *mut u8, cap: i32) -> i32;
    fn probe_link_write(link: *mut c_void, data: *const u8, size: i32) -> bool;
    fn probe_link_close(link: *mut c_void);

    // The BMP core and its packet observers (hosted/src/gdb_decoder.cpp).
    fn rngdbstub_init();
    fn rngdbstub_shutdown();
    fn rngdbstub_poll();
    fn rngdbstub_run(len: usize, data: *const u8);
    fn gdb_decode_incoming_c(data: *const u8, size: u32);
    fn gdb_decode_outgoing_c(data: *const u8, size: u32);
}

/// Qt reports the events with the `lnSocketEvent` numbering.
extern "C" fn qt_socket_event(event: u32, arg: *mut c_void) {
    let listener = unsafe { &*(arg as *const SocketListener) };
    let evt = match event {
        1 => SocketEvent::Connected,
        2 => SocketEvent::Disconnect,
        3 => SocketEvent::DataAvailable,
        4 => SocketEvent::WriteAvailable,
        5 => SocketEvent::Error,
        _ => return,
    };
    listener.notify(evt);
}

/// The listening socket of the local GDB client, over Qt.
pub struct QtSocket {
    handle: *mut c_void,
    /// The listener leaves the runner, so it is boxed and Qt keeps the pointer;
    /// the box is released with the socket.
    listener: *mut SocketListener,
}

impl Socket for QtSocket {
    fn create(port: u16, listener: SocketListener) -> Result<Self, SocketError> {
        let listener = Box::into_raw(Box::new(listener));
        let handle = unsafe { ln_socket_open(port, qt_socket_event, listener as *mut c_void) };
        if handle.is_null() {
            unsafe { drop(Box::from_raw(listener)) };
            return Err(SocketError::CreationFailed);
        }
        Ok(Self { handle, listener })
    }

    /// Qt accepts in its own `newConnection` slot.
    fn accept(&mut self) -> Result<(), SocketError> {
        Ok(())
    }

    fn disconnect_client(&mut self) -> Result<(), SocketError> {
        unsafe { ln_socket_disconnect_client(self.handle) };
        Ok(())
    }

    fn read_packet(&mut self, buf: &mut [u8]) -> Result<Option<usize>, SocketError> {
        let mut len: u32 = 0;
        if unsafe { ln_socket_read(self.handle, buf.as_mut_ptr(), buf.len() as u32, &mut len) } != 0 || len == 0 {
            return Ok(None);
        }
        Ok(Some(len as usize))
    }

    fn write(&mut self, data: &[u8]) -> Result<usize, SocketError> {
        let mut done: u32 = 0;
        if unsafe { ln_socket_write(self.handle, data.len() as u32, data.as_ptr(), &mut done) } != 0 {
            return Err(SocketError::WriteFailed);
        }
        Ok(done as usize)
    }

    fn write_buffer_available(&mut self) -> usize {
        let mut len: u32 = 0;
        if unsafe { ln_socket_write_buffer_available(self.handle, &mut len) } != 0 {
            return 0;
        }
        len as usize
    }

    fn flush(&mut self) -> Result<(), SocketError> {
        unsafe { ln_socket_flush(self.handle) };
        Ok(())
    }
}

impl Drop for QtSocket {
    fn drop(&mut self) {
        unsafe {
            ln_socket_close(self.handle);
            drop(Box::from_raw(self.listener));
        }
    }
}

/// Serves the session with the BMP core (USB probe back end).
pub struct GdbHandler;

impl SocketRunnerHandler for GdbHandler {
    fn on_connected(&mut self) {
        unsafe { rngdbstub_init() };
    }

    fn on_disconnected(&mut self) {
        unsafe { rngdbstub_shutdown() };
    }

    fn on_poll(&mut self) {
        unsafe { rngdbstub_poll() };
    }

    /// The loop drains the socket itself: it needs the buffer for `rngdbstub_run`.
    fn on_incoming_data(&mut self) {}
}

/// Longest probe host name kept for the relay.
const HOST_MAX: usize = 128;

/// Relays the session to a network probe (network back end).
pub struct RelayHandler {
    host: [u8; HOST_MAX],
    host_len: usize,
    probe_port: u16,
    link: *mut c_void,
}

impl RelayHandler {
    fn new(host: &[u8], probe_port: u16) -> Self {
        let mut name = [0u8; HOST_MAX];
        let len = host.len().min(HOST_MAX - 1);
        name[..len].copy_from_slice(&host[..len]);
        Self {
            host: name,
            host_len: len,
            probe_port,
            link: core::ptr::null_mut(),
        }
    }

    fn link(&self) -> *mut c_void {
        self.link
    }
}

impl SocketRunnerHandler for RelayHandler {
    fn on_connected(&mut self) {
        let mut name = [0i8; HOST_MAX + 1];
        for (dst, src) in name.iter_mut().zip(self.host.iter()) {
            *dst = *src as i8;
        }
        name[self.host_len] = 0;
        // Nothing to relay to: the caller drops the client (see `RelayHandler::connected`).
        self.link = unsafe { probe_link_open(name.as_ptr(), self.probe_port) };
    }

    fn on_disconnected(&mut self) {
        if !self.link.is_null() {
            unsafe { probe_link_close(self.link) };
            self.link = core::ptr::null_mut();
        }
    }

    fn on_poll(&mut self) {}

    fn on_incoming_data(&mut self) {}
}

/* ------------------------------------------------------------------------- */
/*  The two back ends, behind one trait the C ABI drives                     */
/* ------------------------------------------------------------------------- */

trait Backend {
    fn start(&mut self) -> bool;
    fn process(&mut self, events: u32);
    /// Drain the client socket and, for the relay, the probe link.
    fn poll(&mut self);
    fn is_connected(&self) -> bool;
    fn disconnect(&mut self);
    fn write(&mut self, data: &[u8]);
    fn flush(&mut self);
}

/// Replies are staged here while the BMP core runs: `rngdb_send_data_c()` is
/// called from inside `rngdbstub_run()`, which already borrows the runner.
struct OutBox {
    data: [u8; OUT_MAX],
    len: usize,
}

static OUT: SyncCell<OutBox> = SyncCell::new(OutBox {
    data: [0; OUT_MAX],
    len: 0,
});

fn stage_output(data: &[u8]) {
    let out = unsafe { OUT.as_mut() };
    let room = OUT_MAX - out.len;
    let len = data.len().min(room);
    out.data[out.len..out.len + len].copy_from_slice(&data[..len]);
    out.len += len;
}

fn flush_staged<B: Backend + ?Sized>(backend: &mut B) {
    let out = unsafe { OUT.as_mut() };
    if out.len == 0 {
        return;
    }
    let len = out.len;
    out.len = 0;
    backend.write(&out.data[..len]);
    backend.flush();
}

impl Backend for SocketRunner<GdbHandler, QtSocket> {
    fn start(&mut self) -> bool {
        self.start_server().is_ok()
    }

    fn process(&mut self, events: u32) {
        self.process_events(events);
    }

    fn poll(&mut self) {
        let mut buf = [0u8; RX_CHUNK];
        while let Ok(Some(len)) = self.read_packet(&mut buf) {
            unsafe { gdb_decode_incoming_c(buf.as_ptr(), len as u32) };
            unsafe { rngdbstub_run(len, buf.as_ptr()) };
            flush_staged(self);
        }
        let _ = self.flush();
    }

    fn is_connected(&self) -> bool {
        self.is_connected()
    }

    fn disconnect(&mut self) {
        let _ = self.disconnect_client();
    }

    fn write(&mut self, data: &[u8]) {
        let _ = SocketRunner::write(self, data);
    }

    fn flush(&mut self) {
        let _ = SocketRunner::flush(self);
    }
}

impl Backend for SocketRunner<RelayHandler, QtSocket> {
    fn start(&mut self) -> bool {
        self.start_server().is_ok()
    }

    fn process(&mut self, events: u32) {
        self.process_events(events);
    }

    fn poll(&mut self) {
        let link = self.handler.link();
        if link.is_null() {
            return;
        }
        let mut buf = [0u8; RX_CHUNK];
        // Client -> probe.
        while let Ok(Some(len)) = self.read_packet(&mut buf) {
            unsafe { gdb_decode_incoming_c(buf.as_ptr(), len as u32) };
            if !unsafe { probe_link_write(link, buf.as_ptr(), len as i32) } {
                // The probe link is gone: the session cannot continue.
                let _ = self.disconnect_client();
                return;
            }
            // A reply normally follows straight away: pick it up now.
            pump(self, RELAY_REPLY_WAIT_MS);
        }
        pump(self, 0);
    }

    fn is_connected(&self) -> bool {
        self.is_connected()
    }

    fn disconnect(&mut self) {
        let _ = self.disconnect_client();
    }

    fn write(&mut self, data: &[u8]) {
        let _ = SocketRunner::write(self, data);
    }

    fn flush(&mut self) {
        let _ = SocketRunner::flush(self);
    }
}

/// Move whatever the probe sent into the GDB client socket.
fn pump(runner: &mut SocketRunner<RelayHandler, QtSocket>, wait_ms: i32) {
    let link = runner.handler.link();
    if link.is_null() {
        return;
    }
    let mut buf = [0u8; RX_CHUNK];
    let n = unsafe { probe_link_read(link, wait_ms, buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 0 {
        if !unsafe { probe_link_is_open(link) } {
            let _ = runner.disconnect_client();
        }
        return;
    }
    let n = n as usize;
    unsafe { gdb_decode_outgoing_c(buf.as_ptr(), n as u32) };
    let _ = runner.write(&buf[..n]);
    let _ = runner.flush();
}

/* ------------------------------------------------------------------------- */
/*  C ABI used by the hosted GDB server thread (esp_socket_rust.h)           */
/* ------------------------------------------------------------------------- */
/// One served session, opaque to the C++ thread.
pub struct Hosted {
    backend: Box<dyn Backend>,
}

/// Slot shift of a hosted runner: there is one per executable.
const SHIFT: u32 = 0;

fn hosted<'a>(runner: *mut c_void) -> &'a mut Hosted {
    unsafe { &mut *(runner as *mut Hosted) }
}

#[unsafe(no_mangle)]
pub extern "C" fn hosted_runner_new(
    event_group: *mut c_void,
    port: u16,
    probe: *const c_char,
    probe_port: u16,
) -> *mut c_void {
    if event_group.is_null() {
        return core::ptr::null_mut();
    }
    let event_group = unsafe { EventGroup::from_raw(event_group) };
    let backend: Box<dyn Backend> = if probe.is_null() {
        Box::new(SocketRunner::new(port, event_group, SHIFT, GdbHandler))
    } else {
        let host = unsafe { core::ffi::CStr::from_ptr(probe) }.to_bytes();
        Box::new(SocketRunner::new(
            port,
            event_group,
            SHIFT,
            RelayHandler::new(host, probe_port),
        ))
    };
    Box::into_raw(Box::new(Hosted { backend })) as *mut c_void
}

#[unsafe(no_mangle)]
pub extern "C" fn hosted_runner_start(runner: *mut c_void) -> bool {
    hosted(runner).backend.start()
}

#[unsafe(no_mangle)]
pub extern "C" fn hosted_runner_process(runner: *mut c_void, events: u32) {
    hosted(runner).backend.process(events)
}

#[unsafe(no_mangle)]
pub extern "C" fn hosted_runner_poll(runner: *mut c_void) {
    hosted(runner).backend.poll()
}

#[unsafe(no_mangle)]
pub extern "C" fn hosted_runner_is_connected(runner: *mut c_void) -> bool {
    hosted(runner).backend.is_connected()
}

#[unsafe(no_mangle)]
pub extern "C" fn hosted_runner_disconnect(runner: *mut c_void) {
    hosted(runner).backend.disconnect()
}

#[unsafe(no_mangle)]
pub extern "C" fn hosted_runner_write(runner: *mut c_void, data: *const u8, len: u32) {
    if data.is_null() {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(data, len as usize) };
    hosted(runner).backend.write(data)
}

#[unsafe(no_mangle)]
pub extern "C" fn hosted_runner_flush(runner: *mut c_void) {
    hosted(runner).backend.flush()
}

#[unsafe(no_mangle)]
pub extern "C" fn hosted_runner_free(runner: *mut c_void) {
    if !runner.is_null() {
        unsafe { drop(Box::from_raw(runner as *mut Hosted)) };
    }
}

/// The BMP core emits its GDB replies here; they are staged and written after
/// the run that produced them (`rngdbstub_run` already borrows the runner).
#[unsafe(no_mangle)]
pub extern "C" fn rngdb_send_data_c(size: u32, ptr: *const u8) {
    if ptr.is_null() {
        return;
    }
    let data = unsafe { core::slice::from_raw_parts(ptr, size as usize) };
    unsafe { gdb_decode_outgoing_c(data.as_ptr(), size) };
    stage_output(data);
}

/// Output is written at the end of every poll tick.
#[unsafe(no_mangle)]
pub extern "C" fn rngdb_output_flush_c() {}


