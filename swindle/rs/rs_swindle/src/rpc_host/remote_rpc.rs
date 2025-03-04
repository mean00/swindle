/**
 * This file is used by the host in bmpa mode
 * It is NOT used by lnBMP as a standalone probe
 */
use crate::rpc_host::remote_encoder::*;

use crate::bmplogger::*;
use crate::parsing_util::ascii_hex_string_to_u8s;
use crate::parsing_util::ascii_octet_to_hex;
use crate::parsing_util::u8s_string_to_u32_le;
use crate::rpc_common::*;
use crate::rpc_host::remote_encoder::*;

crate::setup_log!(false);

crate::gdb_print_init!();
use crate::{bmplog, bmpwarning, gdb_print};

use crate::rn_bmp_cmd_c::platform_buffer_read;

const RPC_BUFFER_SIZE: usize = 512;

static mut rpc_buffer: [u8; RPC_BUFFER_SIZE] = [0; RPC_BUFFER_SIZE];

//_______________________________________
#[unsafe(no_mangle)]
pub fn remote_bmp_set_frequency_rs(fq: u32) -> bool {
    let mut e = rpc_encoder::new();
    e.begin();
    e.add_u8(&[RPC_SWINDLE_PACKET, RPC_SWINDLE_SET_FQ]);
    e.add_u32_le(fq);
    e.end();
    let reply = remote_get_reply();
    check_reply(reply, 8)
}
//_______________________________________
#[unsafe(no_mangle)]
pub fn remote_bmp_get_frequency_rs(fq: &mut u32) -> bool {
    let mut e = rpc_encoder::new();
    e.begin();
    e.add_u8(&[RPC_SWINDLE_PACKET, RPC_SWINDLE_GET_FQ]);
    e.end();
    let reply = remote_get_reply();
    if !check_reply(reply, 8) {
        return false;
    }
    *fq = u8s_string_to_u32_le(&reply[1..]);
    true
}
#[unsafe(no_mangle)]
pub fn bmp_set_frequency_c(fq: u32) {
    remote_bmp_set_frequency_rs(fq);
}
#[unsafe(no_mangle)]
pub fn bmp_get_frequency_c() -> u32 {
    let mut fq: u32 = 0;
    remote_bmp_get_frequency_rs(&mut fq);
    fq
}

//-------------------------------
fn remote_get_reply() -> &'static [u8] {
    unsafe {
        let buf: *mut u8 = rpc_buffer.as_mut_ptr();
        let se = platform_buffer_read(buf, RPC_BUFFER_SIZE as i32);
        &rpc_buffer[0..(se as usize)]
    }
}
/**
 *
 */
fn check_reply(reply: &[u8], expected: usize) -> bool {
    if reply.len() == 0 {
        return false;
    }
    if reply[0] != RPC_RESP_OK {
        return false;
    }
    if reply.len() != (expected + 1) {
        return false;
    }
    true
}

#[unsafe(no_mangle)]
pub fn remote_ch32_riscv_dmi_reset_rs() -> bool {
    let mut e = rpc_encoder::new();
    e.begin();
    e.add_u8(&[RPC_RV_PACKET, RPC_RV_RESET]);
    e.end();
    // now get the reply
    let reply = remote_get_reply();
    if !check_reply(reply, 8) {
        return false;
    }
    true
}

/**
 *
 */
#[unsafe(no_mangle)]
pub fn remote_ch32_riscv_dmi_read_rs(address: u32, value: &mut u32) -> bool {
    let mut e = rpc_encoder::new();
    e.begin();
    e.add_u8(&[RPC_RV_PACKET, RPC_RV_DM_READ]);
    e.add_u32_le(address);
    e.end();
    // now get the reply
    let reply = remote_get_reply();
    if !check_reply(reply, 8) {
        return false;
    }
    *value = u8s_string_to_u32_le(&reply[1..]);
    true
}

/**
 *
 */
#[unsafe(no_mangle)]
pub fn remote_ch32_riscv_dmi_write_rs(address: u32, value: u32) -> bool {
    let mut e = rpc_encoder::new();
    e.begin();
    e.add_u8(&[RPC_RV_PACKET, RPC_RV_DM_WRITE]);
    e.add_u32_le(address);
    e.add_u32_le(value);
    e.end();
    // now get the reply
    let reply = remote_get_reply();
    if !check_reply(reply, 8) {
        return false;
    }
    true
}

#[unsafe(no_mangle)]
pub fn remote_crc32(address: u32, length: u32, out_crc: &mut u32) -> bool {
    let mut e = rpc_encoder::new();
    e.begin();
    e.add_u8(&[RPC_SWINDLE_PACKET, RPC_SWINDLE_CRC32]);
    e.add_u32_le(address);
    e.add_u32_le(length);
    e.end();
    // now get the reply
    let reply = remote_get_reply();
    if !check_reply(reply, 8) {
        return false;
    }
    *out_crc = u8s_string_to_u32_le(&reply[1..]);
    true
}
//-------------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub fn remote_adiv5_swd_write_no_check_rs(addr: u16, data: u32) -> bool {
    bmplog!("lnAdiv5SwdWrite:Addr{:x}:Data{:x}\n", addr, data);
    let mut e = rpc_encoder::new();
    e.begin();
    e.add_u8(&[RPC_LNADIV_PACKET, RPC_LNADIV_WRITE]);
    e.add_u32_le(addr as u32);
    e.add_u32_le(data);
    e.end();
    // now get the reply
    let reply = remote_get_reply();
    if !check_reply(reply, 8) {
        gdb_print!("Incorret reply to adiv_swd_write\n");
        return false;
    }
    true
}
#[unsafe(no_mangle)]
//---------------------
pub fn remote_raw_swd_write_rs(tick: u32, value: u32) {
    bmplog!("lnraw_swd_write:Tick{:x}:Value{:x}\n", tick, value);
    let mut e = rpc_encoder::new();
    e.begin();
    e.add_u8(&[RPC_LNADIV_PACKET, RPC_LNADIV_RAW_WRITE]);
    e.add_u32_le(tick);
    e.add_u32_le(value);
    e.end();
    // now get the reply
    let reply = remote_get_reply();
    if !check_reply(reply, 8) {
        gdb_print!("Incorret reply to adiv_swd_raw_write\n");
    }
}
//---------------------
#[unsafe(no_mangle)]
pub fn remote_adiv5_swd_read_no_check_rs(addr: u16) -> u32 {
    bmplog!("lnadiv5_swd_read:Addr{:x}\n", addr);
    let mut e = rpc_encoder::new();
    e.begin();
    e.add_u8(&[RPC_LNADIV_PACKET, RPC_LNADIV_READ]);
    e.add_u32_le(addr as u32);
    e.end();
    // now get the reply
    let reply = remote_get_reply();
    if !check_reply(reply, 8) {
        gdb_print!("Incorret reply to adiv_swd_read\n");
        return 0;
    }
    u8s_string_to_u32_le(&reply[1..])
}
#[unsafe(no_mangle)]
pub fn remote_adiv5_swd_raw_access_rs(rnw: u8, addr: u16, value: u32, fault: &mut u32) -> u32 {
    bmplog!(
        "lnadiv5_swd_raw_access:rnw{:x}, addr {:x}, value {:x} \n",
        rnw,
        addr,
        value
    );
    let mut e = rpc_encoder::new();
    e.begin();
    e.add_u8(&[RPC_LNADIV_PACKET, RPC_LNADIV_LOWLEVEL]);
    e.add_u32_le(rnw as u32);
    e.add_u32_le(addr as u32);
    e.add_u32_le(value);
    e.end();
    // now get the reply
    let reply = remote_get_reply();
    if !check_reply(reply, 16) {
        gdb_print!("Incorret reply to adiv_swd_raw_access\n");
        *fault = 2;
        return 0;
    }
    *fault = u8s_string_to_u32_le(&reply[9..]);
    u8s_string_to_u32_le(&reply[1..9])
}

//
