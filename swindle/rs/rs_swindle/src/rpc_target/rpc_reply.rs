//
//  This encodes strings to gdb
//  either as extended remote protocol
//  or as hex encoded string
//  aaa=>404040
//
// The code is designed to avoid allocating memory
// AND avoid memcpying that 's why it may look a bit
// complicated
//
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

use crate::encoder::*;
use crate::packet_symbols;
use crate::parsing_util::{u8_to_ascii, u8_to_ascii_to_buffer};
use crate::rn_bmp_cmd_c::platform_buffer_write_buffered;
use crate::rn_bmp_cmd_c::platform_write_flush;
use crate::rpc_common::*;
use crate::{rngdb_output_flush, rngdb_send_data, rngdb_send_data_u8};
use core::ptr::addr_of_mut;

// DF -- not thread safe, not re-entrant,ugly but simple
const REPLY_TEMP_BUFFER_SIZE: usize = 32;
static mut temp_buffer: [u8; REPLY_TEMP_BUFFER_SIZE] = [0; REPLY_TEMP_BUFFER_SIZE];

crate::setup_log!(false);
use crate::{bmplog, bmpwarning};

pub struct rpc_reply_encoder {
    data: &'static mut [u8],
    index: usize,
}

// this buffer is NOT thread safe and should be used
// only in a single shot

fn get_temp_buffer() -> &'static mut [u8] {
    unsafe { &mut *addr_of_mut!(temp_buffer) }
}
//
//
impl rpc_reply_encoder {
    //
    pub fn new() -> Self {
        Self::new_raw(RPC_RESP_OK)
    }
    //
    pub fn new_error() -> Self {
        Self::new_raw(RPC_RESP_ERR)
    }
    //
    fn new_raw(code: u8) -> Self {
        let s = rpc_reply_encoder {
            data: get_temp_buffer(),
            index: 2,
        };
        s.data[0] = RPC_REMOTE_RESP;
        s.data[1] = code;
        s
    }
    //
    pub fn end(&mut self) {
        self.add_u8(crate::packet_symbols::RPC_END);
        encoder::raw_send_u8(&self.data[0..self.index]);
        encoder::flush();
    }
    //
    pub fn add_u32_be_wrapped(&mut self, val: u32) {
        for i in 0..4 {
            let ascii = crate::parsing_util::u8_to_ascii(((val >> (8 * (3 - i))) & 0xff) as u8);
            self.add_u8s(&ascii);
        }
    }
    //
    pub fn add_u32_le_wrapped(&mut self, val: u32) {
        let mut val = val;
        for _i in 0..4 {
            let ascii = crate::parsing_util::u8_to_ascii((val & 0xff) as u8);
            val >>= 8;
            self.add_u8s(&ascii);
        }
    }
    //
    pub fn add_u8_wrapped(&mut self, byt: u8) {
        self.add_u8s(&crate::parsing_util::u8_to_ascii(byt));
    }
    //
    pub fn add_u8s_wrapped(&mut self, byt: &[u8]) {
        for &i in byt {
            self.add_u8_wrapped(i)
        }
    }
    //
    fn add_u8(&mut self, byt: u8) {
        self.data[self.index] = byt;
        self.index += 1;
    }
    //
    pub fn add_string(&mut self, byt: &[u8]) {
        self.add_u8s(byt);
    }
    //
    fn add_u8s(&mut self, byt: &[u8]) {
        for &i in byt {
            self.data[self.index] = i;
            self.index += 1;
        }
    }
    //
    pub fn simple_error(fault: u8) {
        let mut e: rpc_reply_encoder = rpc_reply_encoder::new_raw(RPC_RESP_ERR);
        e.add_u32_le_wrapped(((fault as u32) << 8) + (RPC_ERROR_FAULT as u32));
        e.end();
    }
}
//
