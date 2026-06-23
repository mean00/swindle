//! SEGGER RTT (Real-Time Transfer) support.
//!
//! Implements the SEGGER RTT protocol for reading/writing RTT channels on
//! the target. RTT allows bidirectional communication with the target via
//! shared memory buffers, without requiring a dedicated serial port.
//!
//! ## Key features
//!
//! - Polls RTT control block at configurable intervals
//! - Reads up-channel data (target → host) and sends to GDB
//! - Tracks down-channel write room (host → target)
//! - Automatically halts/resumes the target for RTT access when needed
//! - Cortex-M targets can read RTT without halting (no-stop mode)

use crate::bmp;
use crate::commands::run::HaltState;
//use crate::gdb_print;
use crate::setting_keys::{RTT_PERIOD_KEY, RTT_SETTING_KEY};
use crate::settings;
use core::mem::MaybeUninit;
#[cfg(not(feature = "hosted"))]
use rust_esprit::tick_count;
//crate::gdb_print_init!();
//setup_log!(false);

const RTT_SIGNATURE: &[u8] = b"SEGGER RTT\0";
const RTT_SIGNATURE_LEN: usize = 11;
const TRANSFER_BUFFER_SIZE: usize = 512;
const RTT_POLLING_DEFAULT_PERIOD: u32 = 20;
const RTT_POLL_ROUNDUP: u32 = 1 << 16; // wrap time every 64k ms 
//
unsafe extern "C" {
    fn swindle_rtt_send_data_to_host(index: u32, len: u32, data: *const u8);
    fn swindle_rtt_room_available_to_host(index: u32) -> u32;
}
#[allow(dead_code)]
pub extern "C" fn swindle_rtt_room_available_to_device(index: u32) -> u32 {
    swindle_get_rtt().write_room[index as usize]
}

#[repr(C)]
pub struct RttBuffer {
    pub name: u32,         // Pointer to buffer name (null-terminated string)
    pub buffer: u32,       // Pointer to buffer memory
    pub size: u32,         // Buffer size in bytes
    pub write_offset: u32, // Write offset (producer)
    pub read_offset: u32,  // Read offset (consumer)
    pub flags: u32,        // Buffer flags (blocking/non-blocking)
}
impl RttBuffer {
    fn invalidate(&mut self) {
        self.buffer = 0;
        self.size = 0;
    }
    fn is_valid(&self) -> bool {
        self.buffer != 0 && self.size != 0
    }

    fn print(&self) {
        gdb_println!("\t buffer   : ", Hex(self.buffer));
        gdb_println!("\n\t size     : \n", self.size);
        gdb_println!("\n\t write    : \n", self.write_offset);
        gdb_println!("\n\t read     : \n", self.read_offset);
    }
}
#[repr(C)]
pub struct RttControlBlock {
    pub id: [u8; 16], // "S E G G E R  R T T" signature
    pub max_num_up_buffers: u32,
    pub max_num_down_buffers: u32,
}
const BUFFER_SIZE: u32 = core::mem::size_of::<RttBuffer>() as u32;
const HEADER_SIZE: u32 = core::mem::size_of::<RttControlBlock>() as u32;
//
// To read the RTT info, should the chip be stopped
//
static mut need_stop_flag: bool = true;
/// Check if the target must be halted for RTT buffer access.
///
/// Cortex-M targets can read RTT without halting. Other architectures
/// (e.g. RISC-V) require the target to be stopped.
fn need_stop() -> bool {
    unsafe { need_stop_flag }
}

/// Result of attempting to halt the target for RTT access.
#[derive(PartialEq, Eq)]
enum RttHalt {
    AlreadyHalted,
    Stepping,
    Halted,
    Failure,
}

fn get_tick() -> u32 {
    #[cfg(not(feature = "hosted"))]
    return tick_count() & (RTT_POLL_ROUNDUP - 1);

    #[cfg(feature = "hosted")]
    0
}

/// Halt the target (if needed) for RTT buffer access.
///
/// Returns the halt state so the caller can resume the target later.
fn swindle_rtt_access_to_target() -> RttHalt {
    if !need_stop() {
        return RttHalt::Halted;
    }

    let mut retries = 20;
    let mut proceed = false;
    while retries > 0 {
        let state = bmp::bmp_poll();
        if state == HaltState::Request {
            retries -= 1;
        } else {
            proceed = true;
            break;
        }
    }
    if !proceed {
        return RttHalt::Failure;
    }
    match bmp::bmp_poll() {
        HaltState::Request => RttHalt::Failure,
        HaltState::Fault => {
            //gdb_print!("Target is in fault state, halting for RTT access!\n");
            RttHalt::Failure
        }
        HaltState::Running => {
            //gdb_print!("Target is running, halting for RTT access!\n");
            if !bmp::bmp_target_halt() {
                return RttHalt::Failure;
            }
            RttHalt::Halted
        }
        HaltState::Watchpoint(_) => RttHalt::AlreadyHalted,
        HaltState::Breakpoint => RttHalt::AlreadyHalted,
        HaltState::Stepping => {
            //gdb_print!("Target is stepping, halting for RTT access!\n");
            RttHalt::Stepping
        }
        HaltState::Error =>
        //gdb_print!
        {
            RttHalt::Failure
        } //_ => {
          //gdb_print!("Target is already halted, proceeding with RTT access!\n");
          //panic!("oops : rtt halt\n");
          //}
    }
}
/// Resume the target after RTT access, if it was halted by us.
fn swindle_rtt_release_target(halt: RttHalt) -> bool {
    if !need_stop() {
        return true;
    }
    match halt {
        RttHalt::AlreadyHalted => true,
        RttHalt::Stepping => true,
        RttHalt::Halted => {
            if !bmp::bmp_halt_resume(false) {
                gdb_print!("Failed to resume target after RTT access!\n");
                return false;
            }
            true
        }
        RttHalt::Failure => {
            panic!("Should not try to resumt \n");
        }
    }
}
/// Read the RTT control block from target memory.
impl RttControlBlock {
    /// Read the RTT control block from target memory at `address`.
    pub fn read(address: u32) -> Self {
        let mut block: RttControlBlock = RttControlBlock {
            id: [0; 16],
            max_num_up_buffers: 0,
            max_num_down_buffers: 0,
        };
        let halted = swindle_rtt_access_to_target();
        if halted != RttHalt::Failure {
            bmp::bmp_read_mem_ptr(address, HEADER_SIZE, &mut block as *mut _ as *mut u8);
            swindle_rtt_release_target(halted);
        }
        block
    }
    /// Check if the control block has a valid SEGGER RTT signature.
    pub fn is_valid(&self) -> bool {
        if self.max_num_up_buffers == 0 || self.max_num_up_buffers > 4 {
            return false;
        }
        if self.max_num_down_buffers > 4 {
            return false;
        }
        // check signature
        self.id[0..RTT_SIGNATURE_LEN] == *RTT_SIGNATURE
    }
}

/// Internal RTT state: enabled flag, last poll time, and write room per channel.
struct SeggerRTT {
    enabled: bool,
    last_time: u32,
    write_room: [u32; 4],
}
// SAFETY: SeggerRTT is only used in a single-threaded debugger context.
unsafe impl Sync for SeggerRTT {}
static mut RTT_BUFFER: [u32; TRANSFER_BUFFER_SIZE / 4 + 1] = [0; TRANSFER_BUFFER_SIZE / 4 + 1];
fn get_transfer_buffer() -> *mut u8 {
    unsafe { &mut RTT_BUFFER as *mut _ as *mut u8 }
}

//
fn swindle_get_rtt() -> &'static mut SeggerRTT {
    static mut RTT: MaybeUninit<SeggerRTT> = MaybeUninit::uninit();
    static mut RTT_INIT: bool = false;
    unsafe {
        if !RTT_INIT {
            RTT.write(SeggerRTT {
                enabled: false,
                last_time: 0,
                write_room: [0, 0, 0, 0],
            });
            RTT_INIT = true;
        }
        RTT.assume_init_mut()
    }
}
/// Initialise the RTT subsystem (currently a no-op).
#[unsafe(no_mangle)]
pub fn swindle_init_rtt() {}
/// Reinitialise RTT: reset write room counters.
#[unsafe(no_mangle)]
pub fn swindle_reinit_rtt() {
    swindle_get_rtt().write_room = [0, 0, 0, 0];
}

/// Check if RTT is currently enabled.
#[unsafe(no_mangle)]
pub fn swindle_rtt_enabled() -> bool {
    swindle_get_rtt().enabled
}
/// Poll RTT channels and transfer data (called periodically from C).
///
/// Returns `true` if any data was transferred.
#[unsafe(no_mangle)]
pub extern "C" fn swindle_run_rtt() -> bool {
    swindle_get_rtt().process()
}

/// Purge RTT buffers (currently a no-op).
#[unsafe(no_mangle)]
pub extern "C" fn swindle_purge_rtt() -> bool {
    // swindle_run_rtt();
    false
}

/// Enable or disable RTT polling.
///
/// When enabling, reads the RTT control block address from settings.
/// Determines whether the target needs to be halted for RTT access
/// based on the architecture (Cortex-M can do no-stop RTT).
#[unsafe(no_mangle)]
pub extern "C" fn swindle_enable_rtt(enable: bool) {
    if enable {
        let adr = settings::get_or_default(RTT_SETTING_KEY, 0);
        if adr == 0 {
            gdb_print!("We dont have the RTT symbol available! \n");
            return;
        }
    }
    // For the moment we use a very simple scheme
    // No need to stop for all cortexM
    // stop for all others
    if enable {
        unsafe {
            need_stop_flag = !(crate::bmp::bmp_get_arch() == crate::bmp::bmp_arch::BMP_ARCH_ARM);
        }
        if unsafe { need_stop_flag } {
            gdb_print!("RTT needs to stop the CPU to read \n");
        } else {
            gdb_print!("RTT does NOT need to stop the CPU to read \n");
        }
    } else {
        unsafe {
            need_stop_flag = true;
        }
    }
    swindle_get_rtt().enabled = enable;
}
/// Get the available write room on a down-channel.
#[unsafe(no_mangle)]
pub extern "C" fn swindle_rtt_write_available(channel: u32) -> u32 {
    swindle_get_rtt().write_room[channel as usize]
}
/// Print RTT control block and buffer information to the GDB console.
#[unsafe(no_mangle)]
pub fn swindle_rtt_print_info() {
    let adr = settings::get_or_default(RTT_SETTING_KEY, 0);
    if adr == 0 {
        gdb_print!("We dont have the RTT symbol available! \n");
        return;
    }
    gdb_println!("Checking control block at  address: ", Hex(adr));
    gdb_println!("\n");

    let cb = RttControlBlock::read(adr);

    if cb.max_num_up_buffers == 0 || cb.max_num_up_buffers > 4 {
        gdb_println!("Invalid number of up buffers ", cb.max_num_up_buffers);
        return;
    }
    if cb.max_num_down_buffers > 4 {
        gdb_println!("Invalid number of down buffers ", cb.max_num_down_buffers);
        return;
    }
    // check signature
    if cb.id[0..RTT_SIGNATURE_LEN] != *RTT_SIGNATURE {
        gdb_print!("Invalid signature \n");
        return;
    }
    gdb_println!("RTT control block   :");
    gdb_println!("RTT address         : ", Hex(adr));
    gdb_println!("RTT # up channels   :", cb.max_num_up_buffers);
    gdb_println!("RTT # down channels :", cb.max_num_up_buffers);
    let mut adr_buf = adr + HEADER_SIZE;
    let mut buffer: RttBuffer = RttBuffer {
        name: 0,
        buffer: 0,
        size: 0,
        write_offset: 0,
        read_offset: 0,
        flags: 0,
    };
    for i in 0..cb.max_num_up_buffers {
        buffer.invalidate();
        gdb_println!("Up Channel  ", i);
        if SeggerRTT::read_buffer(adr_buf, &mut buffer) && buffer.is_valid() {
            buffer.print();
            // process up
        } else {
            gdb_print!("  No buffer found!\n");
            continue;
        }
        adr_buf += BUFFER_SIZE;
    }
    for i in 0..cb.max_num_down_buffers {
        buffer.invalidate();
        gdb_println!("Down Channel  ", i);
        if SeggerRTT::read_buffer(adr_buf, &mut buffer) && buffer.is_valid() {
            buffer.print();
            // process up
        } else {
            gdb_print!("  No buffer found!\n");
            continue;
        }
        adr_buf += BUFFER_SIZE;
    }
}
/// Write data to an RTT down-channel (host → target).
///
/// Currently a stub — always returns `true` without writing.
#[unsafe(no_mangle)]
pub extern "C" fn swindle_write_rtt_channel(_channel: u32, _size: u32, _data: *const u8) -> bool {
    /*
        if buffer.write_offset == buffer.read_offset || buffer.size == 0 || buffer.buffer == 0 {
            return false;
        }
        let mut chunk: u32;
        if buffer.read_offset < buffer.write_offset {
            chunk = buffer.write_offset - buffer.read_offset;
        } else {
            chunk = buffer.size - buffer.read_offset;
        }
        if chunk > (TRANSFER_BUFFER_SIZE as u32) {
            chunk = TRANSFER_BUFFER_SIZE as u32;
        }
        if chunk > available {
            chunk = available;
        }
        if chunk == 0 {
            return false;
        }
        let extra = (buffer.buffer + buffer.read_offset) & 3;
        let extra_chunk = (chunk + 3 + extra) & !3;
        let extra_address = (buffer.buffer + buffer.read_offset) & !3;
        let halted = swindle_rtt_access_to_target();
        if halted == RttHalt::Failure {
            gdb_print!("Failed to access target for RTT read!\n");
            return false;
        }

        bmp::bmp_read_mem_ptr(
            // Read aligned ..
            extra_address,
            extra_chunk,
            get_transfer_buffer(),
        );
        let mut new_read = buffer.read_offset + chunk;
        if new_read == buffer.size {
            new_read = 0;
        }

        //
        let updated: [u32; 1] = [new_read];
        bmp::bmp_write_mem32(address + 16, &updated);
        swindle_rtt_release_target(halted);
        // do something with it
        let offset: usize = extra as usize;
        unsafe {
            swindle_rtt_send_data_to_host(
                index as u32,
                chunk,
                RTT_BUFFER[offset..(offset + 1)].as_ptr(),
            );
        }
        // the usable part is RTT_BUFFER[ extra, (extra+chunk)]
        // TODO
    */
    true
}
/// Read data from an RTT up-channel (target → host).
///
/// Reads available data from the target's RTT buffer and sends it to
/// the host via `swindle_rtt_send_data_to_host`. Handles buffer wrap-around
/// and alignment.
#[unsafe(no_mangle)]
pub extern "C" fn swindle_read_rtt_channel(
    index: usize,
    buffer: &RttBuffer,
    address: u32,
    available: u32,
) -> bool {
    if buffer.write_offset == buffer.read_offset || buffer.size == 0 || buffer.buffer == 0 {
        return false;
    }
    let mut chunk: u32;
    if buffer.read_offset < buffer.write_offset {
        chunk = buffer.write_offset - buffer.read_offset;
    } else {
        chunk = buffer.size - buffer.read_offset;
    }
    if chunk > (TRANSFER_BUFFER_SIZE as u32) {
        chunk = TRANSFER_BUFFER_SIZE as u32;
    }
    if chunk > available {
        chunk = available;
    }
    if chunk == 0 {
        return false;
    }
    let extra = (buffer.buffer + buffer.read_offset) & 3;
    let extra_chunk = (chunk + 3 + extra) & !3;
    let extra_address = (buffer.buffer + buffer.read_offset) & !3;
    let halted = swindle_rtt_access_to_target();
    if halted == RttHalt::Failure {
        gdb_print!("Failed to access target for RTT read!\n");
        return false;
    }

    bmp::bmp_read_mem_ptr(
        // Read aligned ..
        extra_address,
        extra_chunk,
        get_transfer_buffer(),
    );
    let mut new_read = buffer.read_offset + chunk;
    if new_read == buffer.size {
        new_read = 0;
    }

    //
    let updated: [u32; 1] = [new_read];
    bmp::bmp_write_mem32(address + 16, &updated);
    swindle_rtt_release_target(halted);
    // do something with it
    let offset: usize = extra as usize;
    unsafe {
        let ptr = get_transfer_buffer();
        swindle_rtt_send_data_to_host(index as u32, chunk, ptr.add(offset));
    }
    // the usable part is RTT_BUFFER[ extra, (extra+chunk)]
    // TODO
    true
}
impl SeggerRTT {
    /// Poll all RTT channels and transfer data.
    ///
    /// Called periodically from `swindle_run_rtt()`. Respects the polling
    /// period setting to avoid excessive target access.
    fn process(&mut self) -> bool {
        if !self.enabled {
            self.last_time = 0;
            return false;
        }
        // prevent too quick probe 10 ms poll by default
        let mut current_time = get_tick();
        let last_time = self.last_time;
        while current_time < last_time {
            current_time += RTT_POLL_ROUNDUP;
        }
        let period = settings::get_or_default(RTT_PERIOD_KEY, RTT_POLLING_DEFAULT_PERIOD);

        if (current_time - last_time) < period {
            return false;
        }
        self.last_time = current_time;

        let adr = settings::get_or_default(RTT_SETTING_KEY, 0);
        if adr == 0 {
            return false;
        }
        let cb = RttControlBlock::read(adr);
        if !cb.is_valid() {
            return false;
        }
        let mut buffer: RttBuffer = RttBuffer {
            name: 0,
            buffer: 0,
            size: 0,
            write_offset: 0,
            read_offset: 0,
            flags: 0,
        };
        let mut adr_buf: u32 = adr + HEADER_SIZE;
        for index in 0..cb.max_num_up_buffers {
            let available = unsafe { swindle_rtt_room_available_to_host(index) };
            if available > 4 && Self::read_buffer(adr_buf, &mut buffer) {
                swindle_read_rtt_channel(index as usize, &buffer, adr_buf, available);
            }
            adr_buf += BUFFER_SIZE;
        }
        let mut top = cb.max_num_down_buffers;
        if top > 4 {
            top = 4;
        }
        for index in 0..top {
            let mut available: u32 = 0;
            if Self::read_buffer(adr_buf, &mut buffer) {
                let used = if buffer.write_offset >= buffer.read_offset {
                    buffer.write_offset - buffer.read_offset
                } else {
                    buffer.size - (buffer.read_offset - buffer.write_offset)
                };
                available = buffer.size.saturating_sub(used).saturating_sub(1);
            }
            swindle_get_rtt().write_room[index as usize] = available;
            adr_buf += BUFFER_SIZE;
        }
        true
    }
    /// Read an RTT buffer descriptor from target memory.
    pub fn read_buffer(address: u32, buffer: &mut RttBuffer) -> bool {
        let halted = swindle_rtt_access_to_target();
        if halted == RttHalt::Failure {
            gdb_print!("Failed to access target for RTT read!\n");
            return false;
        }
        let r = bmp::bmp_read_mem_ptr(address, BUFFER_SIZE, buffer as *mut _ as *mut u8);
        swindle_rtt_release_target(halted);
        r
    }
}
// EOF
