/*
 *
 */
use crate::bmp;
use crate::gdb_print;
use crate::rn_bmp_cmd_c::{bmp_rtt_get_info_c, bmp_rtt_set_info_c, rttField, rttInfo};
use crate::settings;
crate::gdb_print_init!();
crate::setup_log!(false);

const RTT_SIGNATURE: &[u8] = b"SEGGER RTT\0";
const RTT_SIGNATURE_LEN: usize = 11;
const KEY: &str = "RTT_ADDR"; // FIXME, duplicated
const TRANSFER_BUFFER_SIZE: usize = 512;

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

impl RttControlBlock {
    pub fn read(address: u32) -> Self {
        let mut block: RttControlBlock = RttControlBlock {
            id: [0; 16],
            max_num_up_buffers: 0,
            max_num_down_buffers: 0,
        };
        bmp::bmp_read_mem_ptr(address, HEADER_SIZE, &mut block as *mut _ as *mut u8);
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
static mut RTT_BUFFER: [u8; TRANSFER_BUFFER_SIZE] = [0; TRANSFER_BUFFER_SIZE];

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
pub fn swindle_run_rtt() -> bool {
    swindle_get_rtt().check()
}
/*
*
*/
#[unsafe(no_mangle)]
pub fn swindle_rtt_enable(enabled: bool) {
    if enabled {
        let adr = settings::get_or_default(KEY, 0);
        if adr == 0 {
            gdb_print!("We dont have the RTT symbol available! \n");
        }
        return;
    }
    swindle_get_rtt().enabled = enabled;
}

#[unsafe(no_mangle)]
pub fn swindle_rtt_print_info() {
    let adr = settings::get_or_default(KEY, 0);
    if adr == 0 {
        gdb_print!("We dont have the RTT symbol available! \n");
        return;
    }
    let cb = RttControlBlock::read(adr);
    if !cb.is_valid() {
        gdb_print!("invalid control block! \n");
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
impl SeggerRTT {
    pub fn new() -> Self {
        SeggerRTT { enabled: false }
    }
    fn write_rtt(_index: usize, _buffer: &RttBuffer, _address: u32) -> bool {
        false
    }

    fn read_rtt(_index: usize, buffer: &RttBuffer, address: u32) -> bool {
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
        unsafe {
            bmp::bmp_read_mem_ptr(
                buffer.buffer + buffer.read_offset,
                chunk,
                &mut RTT_BUFFER as *mut _ as *mut u8,
            );
        }
        let mut new_read = buffer.read_offset + chunk;
        if new_read == buffer.size {
            new_read = 0;
        }
        //
        let updated: [u32; 1] = [new_read];
        bmp::bmp_write_mem32(address + 16, &updated);
        // do something with itdd
        // TODO
        return true;
    }
    pub fn check(&mut self) -> bool {
        if false == self.enabled {
            return false;
        }
        let adr = settings::get_or_default(KEY, 0);
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
            if true == Self::read_buffer(adr_buf, &mut buffer) {
                Self::read_rtt(index as usize, &buffer, adr_buf);
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
        bmp::bmp_read_mem_ptr(address, BUFFER_SIZE, buffer as *mut _ as *mut u8)
    }
}
/**
*
*
*
*/
impl rttInfo {
    pub fn new() -> Self {
        rttInfo {
            enabled: 0,
            found: 0,
            min_address: 0,
            max_address: 0,
            min_poll_ms: 0,
            max_poll_ms: 0,
            max_poll_error: 0,
            cb_address: 0,
        }
    }
}
/**
 * Retrieve all rtt stuff in one go
 */
pub fn get_rtt_info() -> rttInfo {
    let mut info = rttInfo::new();
    unsafe {
        bmp_rtt_get_info_c(&mut info);
    }
    info
}
/**
 * Retrieve all rtt stuff in one go
 */
pub fn set_rtt_info(field: rttField, info: &rttInfo) {
    unsafe {
        bmp_rtt_set_info_c(field, info);
    }
}
// EOF
