/*
 *
 */
use crate::bmp;
use crate::gdb_print;
use crate::rn_bmp_cmd_c::{bmp_rtt_get_info_c, bmp_rtt_set_info_c, rttField, rttInfo};
crate::gdb_print_init!();
crate::setup_log!(false);

const RTT_SIGNATURE: &[u8] = b"SEGGER RTT\0";
const RTT_SIGNATURE_LEN: usize = 11;
const KEY: &str = "RTT_ADDR"; // FIXME, duplicated

#[repr(C)]
pub struct RttBuffer {
    pub name: u32,         // Pointer to buffer name (null-terminated string)
    pub buffer: u32,       // Pointer to buffer memory
    pub size: u32,         // Buffer size in bytes
    pub write_offset: u32, // Write offset (producer)
    pub read_offset: u32,  // Read offset (consumer)
    pub flags: u32,        // Buffer flags (blocking/non-blocking)
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

static mut segger_rtt: Option<SeggerRTT> = None;
/*
*
*/
pub fn swindle_run_rtt() -> bool {
    unsafe {
        return match &mut segger_rtt {
            None => {
                let mut s = SeggerRTT::new();
                let r = s.check();
                segger_rtt = Some(s);
                r
            }
            Some(x) => x.check(),
        };
    }
}
/*
*
*/
pub fn swindle_rtt_enable(enabled: bool) {
    if (enabled) {
        let adr = bmp::bmp_get_setting_or_default(KEY, 0);
        if adr == 0 {
            gdb_print!("We dont have the RTT symbol available! \n");
        }
        return;
    }
    unsafe {
        match &mut segger_rtt {
            None => {
                let mut s = SeggerRTT::new();
                s.enabled = enabled;
                segger_rtt = Some(s);
            }
            Some(x) => x.enabled = enabled,
        }
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
    pub fn check(&mut self) -> bool {
        if false == self.enabled {
            return false;
        }
        let adr = bmp::bmp_get_setting_or_default(KEY, 0);
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
                // process up
            }
            adr_buf += BUFFER_SIZE;
        }
        for index in 0..cb.max_num_down_buffers {
            if true == Self::read_buffer(adr_buf, &mut buffer) {
                // process down
            }
            adr_buf += BUFFER_SIZE;
        }
        true
    }
    pub fn read_buffer(address: u32, buffer: &mut RttBuffer) -> bool {
        //:bmp::bmp_read_ptr()
        false
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
