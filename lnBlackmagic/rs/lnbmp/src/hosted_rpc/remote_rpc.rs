/**
 * This file is used by the host in bmpa mode
 * It is NOT used by lnBMP as a standalone probe
 */
use crate::bmp;
use crate::rpc::rpc_commands;
use crate::commands::{exec_one, CallbackType, CommandTree};
use crate::hosted_rpc::remote_encoder::*;

use crate::bmplogger::*;
use crate::parsing_util::ascii_hex_string_to_u8s;
use crate::parsing_util::ascii_octet_to_hex;

use crate::rpc::rpc_commands::*;
/**
 * This handles the low level RPC as used when BMP is running in hosted mode
 * It is a parralel path to the normal gdb command and is BMP specific
 */
use alloc::vec;
use alloc::vec::Vec;

crate::setup_log!(false);
use crate::{bmplog, bmpwarning};

use crate::rn_bmp_cmd_c::platform_buffer_read;


const  RPC_BUFFER_SIZE : usize = 512;

static mut rpc_buffer: [u8;RPC_BUFFER_SIZE] = [0;RPC_BUFFER_SIZE];

//-------------------------------
fn remote_get_reply( ) -> &'static [u8] {
unsafe {
    let buf : *mut u8 = rpc_buffer.as_mut_ptr();
    let se = platform_buffer_read(buf, RPC_BUFFER_SIZE as i32);
     &rpc_buffer[0..(se as usize)]
}
}
/**
 * send rv_dm_probe to target
 */
#[no_mangle]
pub fn remote_rv_dm_probe(id : &mut u32)->bool {

    let mut e = rpc_encoder::new();
    e.begin();
    e.add_u8(&[RPC_RV_PACKET]);
    e.add_u8(&[RPC_RV_SCAN]);
    e.end();
    // now get the reply
    let reply = remote_get_reply();
    if reply.len()!=9
    {
        return false;
    }
    *id =0x11223344;
    true
}
/*

    while left != 0 {
        let chunk: usize = core::cmp::min(16, left);
        crate::bmp::bmp_read_mem(current_address, &mut tmp[0..chunk]);
        left -= chunk;
        for i in 0..chunk {
            crate::parsing_util::u8_to_ascii_to_buffer(tmp[i], &mut char_buffer[(2 * i)..]);
        }
        e.add_u8(&char_buffer[..(2 * chunk)]);
        // avoid overflow
        if left != 0 {
            current_address += chunk as u32;
        }
    }

    let mut e : encoder;

    uint8_t buffer[] = {REMOTE_SOM, RPC_RV_PACKET, RPC_RV_SCAN,  REMOTE_EOM};
    platform_buffer_write(buffer, sizeof(buffer));
    int length = platform_buffer_read(reply, REMOTE_MAX_MSG_SIZE);
    if (length !=9)
    {
        DEBUG_ERROR("rv_dm_probe\n");
        return false;
    }
    bool r = reply[0] == REMOTE_RESP_OK;
    if (!r)
    {
        DEBUG_ERROR(" dm_probe set error\n");
        xAssert(0);
    }
    
    *id  = from_ptr(reply+2);
    return true;

}
*/