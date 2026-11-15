//! Network GDB and RTT handlers backed by `rs_socketrunner`.

extern crate rs_w5500;

use core::ffi::c_void;

use rs_socketrunner::{
    DEFAULT_EVENT_WAIT_TIMEOUT_MS, DEFAULT_GDB_PORT, DEFAULT_RTT_PORT,
    EventGroup, GDB_SLOT_SHIFT, RTT_SLOT_SHIFT, RunnerEvent,
    SocketRunner, SocketRunnerHandler,
};

use crate::bmp;
use crate::rtt;
use crate::SyncCell;

// ---------------------------------------------------------------------------
//  GDB Network Handler
// ---------------------------------------------------------------------------

struct GdbNetHandler {
    connected: bool,
}

impl SocketRunnerHandler for GdbNetHandler {
    fn on_connected(&mut self) {
        self.connected = true;
        #[cfg(any(feature = "rp2040", feature = "rp2350"))]
        unsafe {
            swindle_leds_set_connected(true);
        }
        unsafe {
            bmp_io_begin_session();
            rngdbstub_init();
        }
    }

    fn on_disconnected(&mut self) {
        self.connected = false;
        #[cfg(any(feature = "rp2040", feature = "rp2350"))]
        unsafe {
            swindle_leds_set_connected(false);
        }
        unsafe {
            rngdbstub_shutdown();
            bmp_io_end_session();
        }
    }

    fn on_poll(&mut self) {
        if self.connected {
            unsafe {
                rngdbstub_poll();
            }
        }
    }

    fn on_incoming_data(&mut self) {
        // Drained inside the task loop
    }
}

// ---------------------------------------------------------------------------
//  RTT Network Handler
// ---------------------------------------------------------------------------

struct RttNetHandler {
    connected: bool,
}

impl SocketRunnerHandler for RttNetHandler {
    fn on_connected(&mut self) {
        self.connected = true;
        unsafe {
            swindle_reinit_rtt();
        }
    }

    fn on_disconnected(&mut self) {
        self.connected = false;
    }

    fn on_poll(&mut self) {
        if self.connected && bmp::bmp_attached() {
            if rtt::swindle_rtt_enabled() {
                rtt::swindle_run_rtt();
            } else {
                rtt::swindle_purge_rtt();
            }
        }
    }

    fn on_incoming_data(&mut self) {
        // Drop inbound data
    }
}

// ---------------------------------------------------------------------------
//  Global Runner References for C Redirection (rngdb_send_data_c)
// ---------------------------------------------------------------------------

use rs_w5500::W5500SocketHandle;

type GdbRunner = SocketRunner<GdbNetHandler, W5500SocketHandle>;
type RttRunner = SocketRunner<RttNetHandler, W5500SocketHandle>;

static GDB_RUNNER_PTR: SyncCell<*mut GdbRunner> = SyncCell::new(core::ptr::null_mut());
static RTT_RUNNER_PTR: SyncCell<*mut RttRunner> = SyncCell::new(core::ptr::null_mut());
static EVENT_GROUP_PTR: SyncCell<*mut EventGroup> = SyncCell::new(core::ptr::null_mut());

/// Size of the receive buffer below, i.e. the biggest packet one read can return.
const RX_PACKET_SIZE: usize = 2048;

/// Destination of `Socket::read_packet()`, which copies the received bytes out
/// of the socket.  One buffer is enough: this task drains the GDB socket and then
/// the RTT one, never both at once.
static RX_PACKET: SyncCell<[u8; RX_PACKET_SIZE]> = SyncCell::new([0; RX_PACKET_SIZE]);

#[unsafe(no_mangle)]
pub extern "C" fn rngdb_net_notify_link(up: bool) {
    let eg_ptr = EVENT_GROUP_PTR.get();
    if !eg_ptr.is_null() {
        let eg = unsafe { &*eg_ptr };
        let bit = if up { RunnerEvent::UP.0 } else { RunnerEvent::DOWN.0 };
        eg.set_events(bit);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rngdb_net_set_event(bits: u32) {
    let eg_ptr = EVENT_GROUP_PTR.get();
    if !eg_ptr.is_null() {
        let eg = unsafe { &*eg_ptr };
        eg.set_events(bits);
    }
}

/// Callback matching lnLwipEvent / W5500 event signature:
/// 1 = LwipDown, 2 = LwipReady
#[unsafe(no_mangle)]
pub extern "C" fn NetCb_c(evt: u32, _arg: *mut c_void) {
    match evt {
        1 => rngdb_net_notify_link(false),
        2 => rngdb_net_notify_link(true),
        _ => {},
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rngdb_send_data_c(sz: u32, ptr: *const u8) {
    let runner_ptr = GDB_RUNNER_PTR.get();
    if !runner_ptr.is_null() && !ptr.is_null() {
        let runner = unsafe { &mut *runner_ptr };
        if runner.is_connected() {
            let slice = unsafe { core::slice::from_raw_parts(ptr, sz as usize) };
            let _ = runner.write(slice);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rngdb_output_flush_c() {
    let runner_ptr = GDB_RUNNER_PTR.get();
    if !runner_ptr.is_null() {
        let runner = unsafe { &mut *runner_ptr };
        if runner.is_connected() {
            let _ = runner.flush();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn swindle_rtt_send_data_to_host(_index: u32, len: u32, data: *const u8) {
    let runner_ptr = RTT_RUNNER_PTR.get();
    if !runner_ptr.is_null() && !data.is_null() {
        let runner = unsafe { &mut *runner_ptr };
        if runner.is_connected() {
            let slice = unsafe { core::slice::from_raw_parts(data, len as usize) };
            let _ = runner.write(slice);
            let _ = runner.flush();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn swindle_rtt_room_available_to_host(_dex: u32) -> u32 {
    let runner_ptr = RTT_RUNNER_PTR.get();
    if !runner_ptr.is_null() {
        let runner = unsafe { &mut *runner_ptr };
        if runner.is_connected() {
            return runner.write_buffer_available() as u32;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn gdb_if_init() -> bool {
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn swindleRedirectLog_c(_toggle: i32) {}

#[unsafe(no_mangle)]
pub extern "C" fn debug_serial_send_stdout(data: *const u8, _len: usize) {
    if !data.is_null() {
        unsafe {
            Logger(c"%s".as_ptr() as *const u8, data);
        }
    }
}

// ---------------------------------------------------------------------------
//  Main Network Task Loop
// ---------------------------------------------------------------------------

#[cfg(any(feature = "rp2040", feature = "rp2350"))]
const W5500_SPI_INSTANCE: u32 = 0;

#[cfg(any(feature = "rp2040", feature = "rp2350"))]
fn w5500_pins() -> rs_w5500::W5500Pins {
    use rust_esprit::Pin;
    #[cfg(feature = "rp_zero")]
    {
        rs_w5500::W5500Pins {
            miso: Pin::GPIO4,
            mosi: Pin::GPIO3,
            clk: Pin::GPIO2,
            cs: Pin::GPIO5,
            reset: Pin::GPIO6,
            intr: Pin::GPIO7,
        }
    }
    #[cfg(not(feature = "rp_zero"))]
    {
        rs_w5500::W5500Pins {
            miso: Pin::GPIO16,
            mosi: Pin::GPIO19,
            clk: Pin::GPIO18,
            cs: Pin::GPIO17,
            reset: Pin::GPIO21,
            intr: Pin::GPIO20,
        }
    }
}

#[cfg(any(feature = "rp2040", feature = "rp2350"))]
fn init_w5500() {
    let id = unsafe { ln_cpu_serial_id() }.to_le_bytes();
    let mac = [0x00, 0x08, 0xdc, id[0], id[1], id[2]];
    rs_w5500::begin(
        W5500_SPI_INSTANCE,
        w5500_pins(),
        mac,
        Some(NetCb_c),
        core::ptr::null_mut(),
    );
}

/// Main entry point for the Network GDB & RTT task.
/// Replaces the C++ `gdb_task()` event loop.
#[unsafe(no_mangle)]
pub extern "C" fn rngdb_net_gdb_task() {
    unsafe {
        bmp_gpio_init_once();
    }

    let mut eg = EventGroup::uninit();
    eg.init_static();
    eg.take_ownership();

    EVENT_GROUP_PTR.set(&mut eg as *mut EventGroup);

    #[cfg(any(feature = "rp2040", feature = "rp2350"))]
    init_w5500();

    let mut gdb_runner = SocketRunner::new(
        DEFAULT_GDB_PORT,
        eg.clone(),
        GDB_SLOT_SHIFT,
        GdbNetHandler { connected: false },
    );
    let mut rtt_runner = SocketRunner::new(
        DEFAULT_RTT_PORT,
        eg.clone(),
        RTT_SLOT_SHIFT,
        RttNetHandler { connected: false },
    );

    GDB_RUNNER_PTR.set(&mut gdb_runner as *mut GdbRunner);
    RTT_RUNNER_PTR.set(&mut rtt_runner as *mut RttRunner);

    let wait_mask = 0xFFFF_FFFF;

    // Single writer, single reader: this task owns the receive buffer.
    let rx_packet = unsafe { RX_PACKET.as_mut() };

    loop {
        let events = eg.wait_events(wait_mask, DEFAULT_EVENT_WAIT_TIMEOUT_MS as i32);

        // Process GDB events (process_events expects raw event mask)
        gdb_runner.process_events(events);

        // Drain incoming GDB data
        if gdb_runner.is_connected() {
            loop {
                let drained = gdb_runner
                    .with_incoming_packet(&mut rx_packet[..], |slice| unsafe {
                        rngdbstub_run(slice.len(), slice.as_ptr());
                    })
                    .unwrap_or(None);
                if drained.is_none() {
                    break;
                }
            }
            let _ = gdb_runner.flush();
        }

        // Process RTT events
        rtt_runner.process_events(events);

        // Drain/drop incoming RTT data
        if rtt_runner.is_connected() {
            while let Ok(Some(_)) = rtt_runner.read_packet(&mut rx_packet[..]) {}
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rngdb_net_task() {
    rngdb_net_gdb_task();
}

#[unsafe(export_name = "_Z8gdb_taskPv")]
pub extern "C" fn gdb_task(_param: *mut c_void) {
    rngdb_net_gdb_task();
}

unsafe extern "C" {
    #[cfg(any(feature = "rp2040", feature = "rp2350"))]
    fn swindle_leds_set_connected(connected: bool);
    #[link_name = "\u{1}_Z18bmp_gpio_init_oncev"]
    fn bmp_gpio_init_once();
    #[link_name = "\u{1}_ZN7lnCpuID11getSerialIDEv"]
    fn ln_cpu_serial_id() -> u32;
    fn rngdbstub_run(len: usize, data: *const u8);
    fn rngdbstub_init();
    fn rngdbstub_shutdown();
    fn rngdbstub_poll();
    fn bmp_io_begin_session();
    fn bmp_io_end_session();
    fn swindle_reinit_rtt();
    fn Logger(fmt: *const u8, ...);
}
