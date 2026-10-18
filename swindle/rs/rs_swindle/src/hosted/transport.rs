//! Hosted platform serial transport wrapper.
//!
//! Provides safe Rust abstractions around Blackmagic's C platform buffer
//! I/O (`platform_buffer_read`, `platform_buffer_write_buffered`, `platform_write_flush`).

use crate::rn_bmp_cmd_c;
use crate::rpc_common_generated::RPC_RESP_OK;

pub const RPC_BUFFER_SIZE: usize = 512;
static mut RPC_BUFFER: [u8; RPC_BUFFER_SIZE] = [0; RPC_BUFFER_SIZE];

/// Write data to the hosted serial buffer.
#[inline]
pub fn platform_buffer_write(data: &[u8]) {
    unsafe {
        rn_bmp_cmd_c::platform_buffer_write_buffered(data.as_ptr(), data.len() as i32);
    }
}

/// Flush the hosted serial write buffer.
#[inline]
pub fn platform_buffer_flush() {
    unsafe {
        rn_bmp_cmd_c::platform_write_flush();
    }
}

/// Read an RPC reply into the static reply buffer and return a slice of the received bytes.
pub fn remote_get_reply() -> &'static [u8] {
    unsafe {
        let buf: *mut u8 = RPC_BUFFER.as_mut_ptr();
        let se = rn_bmp_cmd_c::platform_buffer_read(buf, RPC_BUFFER_SIZE as i32);
        if se > 0 {
            &RPC_BUFFER[0..(se as usize)]
        } else {
            &[]
        }
    }
}

/// Validate that an RPC reply has the expected length and status OK.
pub fn check_reply(reply: &[u8], expected: usize) -> bool {
    if reply.is_empty() {
        return false;
    }
    if reply[0] != RPC_RESP_OK {
        return false;
    }
    reply.len() == (expected + 1)
}
