/*
 *
 */
use crate::bmp;
use crate::commands::run::HaltState;
use crate::gdb_print;
use crate::rtt_consts::RTT_SETTING_KEY;
use crate::settings;
crate::gdb_print_init!();
crate::setup_log!(false);

const RTT_SIGNATURE: &[u8] = b"SEGGER RTT\0";
const RTT_SIGNATURE_LEN: usize = 11;
const TRANSFER_BUFFER_SIZE: usize = 512;

unsafe extern "C" {
    fn swindle_rtt_send_data_to_host(index: u32, len: u32, data: *const u8);
    fn swindle_rtt_room_available_to_host(index: u32) -> u32;
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
        gdb_print!("\t buffer   : 0x{:x}\n", self.buffer);
        gdb_print!("\t size     : {}\n", self.size);
        gdb_print!("\t write    : {}\n", self.write_offset);
        gdb_print!("\t read     : {}\n", self.read_offset);
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
/*
*
*/
#[derive(PartialEq, Eq)]
enum RttHalt {
    AlreadyHalted,
    Stepping,
    Halted,
    Failure,
}

fn swindle_rtt_access_to_target() -> RttHalt {
    let mut retries = 20;
    let mut proceed = false;
    while retries > 0 {
        let state = bmp::bmp_poll();
        if state == HaltState::Request {
            retries = retries - 1;
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
/*
*
*/
fn swindle_rtt_release_target(halt: RttHalt) -> bool {
    match halt {
        RttHalt::AlreadyHalted => {
            return true;
        }
        RttHalt::Stepping => {
            return true;
        }
        RttHalt::Halted => {
            if !bmp::bmp_halt_resume(false) {
                gdb_print!("Failed to resume target after RTT access!\n");
                return false;
            }
            return true;
        }
        RttHalt::Failure => {
            panic!("Should not try to resumt \n");
        }
    }
}
/*
*
*/

impl RttControlBlock {
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

/*
*
*
*/
struct SeggerRTT {
    enabled: bool,
}
static mut RTT_BUFFER: [u8; TRANSFER_BUFFER_SIZE + 4] = [0; TRANSFER_BUFFER_SIZE + 4];
fn get_transfer_buffer() -> *mut u8 {
    unsafe { &mut RTT_BUFFER as *mut _ as *mut u8 }
}

static mut segger_rtt: SeggerRTT = SeggerRTT { enabled: false };
//
fn swindle_get_rtt() -> &'static mut SeggerRTT {
    unsafe { &mut segger_rtt }
}
/*
*
*/
#[unsafe(no_mangle)]
pub fn swindle_init_rtt() {}

/*
*
*/
#[unsafe(no_mangle)]
pub fn swindle_rtt_enabled() -> bool {
    swindle_get_rtt().enabled
}
/**
*
*
*/
#[unsafe(no_mangle)]
pub extern "C" fn swindle_run_rtt() -> bool {
    unsafe { segger_rtt.process() }
}

#[unsafe(no_mangle)]
pub extern "C" fn swindle_purge_rtt() -> bool {
    // swindle_run_rtt();
    false
}

/*
*
*/
#[unsafe(no_mangle)]
pub extern "C" fn swindle_enable_rtt(enable: bool) {
    if enable {
        let adr = settings::get_or_default(RTT_SETTING_KEY, 0);
        if adr == 0 {
            gdb_print!("We dont have the RTT symbol available! \n");
            return;
        }
    }
    unsafe {
        segger_rtt.enabled = enable;
    }
}

#[unsafe(no_mangle)]
pub fn swindle_rtt_print_info() {
    let adr = settings::get_or_default(RTT_SETTING_KEY, 0);
    if adr == 0 {
        gdb_print!("We dont have the RTT symbol available! \n");
        return;
    }
    gdb_print!("Checking control block at  address 0x{:x}\n", adr);

    let halted = swindle_rtt_access_to_target();
    if halted == RttHalt::Failure {
        gdb_print!("Failed to access target for Control Block  reading!\n");
        return;
    }
    let cb = RttControlBlock::read(adr);
    swindle_rtt_release_target(halted);

    if cb.max_num_up_buffers == 0 || cb.max_num_up_buffers > 4 {
        gdb_print!("Invalid number of up buffers {}\n", cb.max_num_up_buffers);
        return;
    }
    if cb.max_num_down_buffers > 4 {
        gdb_print!(
            "Invalid number of down buffers {}\n",
            cb.max_num_down_buffers
        );
        return;
    }
    // check signature
    if cb.id[0..RTT_SIGNATURE_LEN] != *RTT_SIGNATURE {
        gdb_print!("Invalid signature \n");
        return;
    }
    gdb_print!("RTT control block   :\n");
    gdb_print!("RTT address         :0x{:x}\n", adr);
    gdb_print!("RTT # up channels   :{}\n", cb.max_num_up_buffers);
    gdb_print!("RTT # down channels :{}\n", cb.max_num_up_buffers);
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
        gdb_print!("Up Channel  {}\n", i);
        if SeggerRTT::read_buffer(adr_buf, &mut buffer) && buffer.is_valid() == true {
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
        gdb_print!("Down Channel  {}\n", i);
        if SeggerRTT::read_buffer(adr_buf, &mut buffer) && buffer.is_valid() == true {
            buffer.print();
            // process up
        } else {
            gdb_print!("  No buffer found!\n");
            continue;
        }
        adr_buf += BUFFER_SIZE;
    }
}

/**
*
*
*/
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
        chunk = (buffer.write_offset - buffer.read_offset) as u32;
    } else {
        chunk = (buffer.size - buffer.read_offset) as u32;
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
    return true;
}
impl SeggerRTT {
    pub fn new() -> Self {
        SeggerRTT { enabled: false }
    }
    fn write_rtt(_index: usize, _buffer: &RttBuffer, _address: u32) -> bool {
        false
    }

    fn process(&mut self) -> bool {
        if !self.enabled {
            return false;
        }
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
            if available > 4 {
                if true == Self::read_buffer(adr_buf, &mut buffer) {
                    swindle_read_rtt_channel(index as usize, &buffer, adr_buf, available);
                }
            }
            adr_buf += BUFFER_SIZE;
        }
        for index in 0..cb.max_num_down_buffers {
            if true == Self::read_buffer(adr_buf, &mut buffer) {
                Self::write_rtt(index as usize, &buffer, adr_buf);
            }
            adr_buf += BUFFER_SIZE;
        }
        true
    }
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
