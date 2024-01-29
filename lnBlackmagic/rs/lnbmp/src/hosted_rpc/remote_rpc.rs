/**
 * This file is used by the host in bmpa mode
 * It is NOT used by lnBMP as a standalone probe
 */
use crate::hosted_rpc::remote_encoder::*;

use crate::bmplogger::*;
use crate::parsing_util::ascii_hex_string_to_u8s;
use crate::parsing_util::ascii_octet_to_hex;
use crate::parsing_util::u8s_string_to_u32_le;
use crate::rpc::rpc_commands::*;

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
 * 
 */
fn check_reply( reply : &[u8], expected : usize) -> bool {
    if reply.len()==0
    {
        return false;
    }
    if reply[0]!= RPC_RESP_OK
    {
        return false;
    }
    if reply.len()!=(expected+1)
    {
        return false;
    }
    true
}

/**
 * send rv_dm_probe to target
 */
#[no_mangle]
pub fn remote_rv_dm_probe(id : &mut u32)->bool {

    let mut e = rpc_encoder::new();
    e.begin();
    e.add_u8(&[RPC_RV_PACKET, RPC_RV_SCAN]);    
    e.end();
    // now get the reply
    let reply = remote_get_reply();
    if  !check_reply(reply,8)    {
        return false;
    }
    *id = u8s_string_to_u32_le( &reply[1..]);
    true
}
/**
 * 
 */
#[no_mangle]
pub fn remote_ch32_riscv_dmi_read_rs( address: u32, value : &mut u32) -> bool {
    let mut e = rpc_encoder::new();
    e.begin();
    e.add_u8(&[RPC_RV_PACKET, RPC_RV_DM_READ]);   
    e.add_u32_le(address);
    e.end();
    // now get the reply
    let reply = remote_get_reply();
    if  !check_reply(reply,8)    {
        return false;
    }    
    *value = u8s_string_to_u32_le( &reply[1..]);
    true
}
/**
 * 
 */
#[no_mangle]
pub fn  remote_ch32_riscv_dmi_write_rs( address: u32, value : u32 ) -> bool {
    let mut e = rpc_encoder::new();
    e.begin();
    e.add_u8(&[RPC_RV_PACKET, RPC_RV_DM_WRITE]);   
    e.add_u32_le(address);
    e.add_u32_le(value);
    e.end();
    // now get the reply
    let reply = remote_get_reply();
    if  !check_reply(reply,8)    {
        return false;
    }    
    true
}
//