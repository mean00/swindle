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

use crate::packet_symbols;
use crate::parsing_util::{u8_to_ascii, u8_to_ascii_to_buffer};
use crate::{rngdb_output_flush, rngdb_send_data, rngdb_send_data_u8};

// DF -- not thread safe, not re-entrant,ugly but simple
const TEMP_BUFFER_SIZE: usize = 64;
static mut temp_buffer: [u8; TEMP_BUFFER_SIZE] = [0; TEMP_BUFFER_SIZE];

crate::setup_log!(false);
use crate::{bmplog, bmpwarning};

pub struct encoder {
    checksum: usize,
    escaped: bool,
}

// this buffer is NOT thread safe and should be used
// only in a single shot

fn get_temp_buffer() -> &'static mut [u8] {
    unsafe { &mut temp_buffer }
}
//
//
impl encoder {
    //
    pub fn new() -> Self {
        encoder {
            checksum: 0,
            escaped: false,
        }
    }
    pub fn reply_bool(status: bool) {
        if status {
            Self::reply_ok();
        } else {
            Self::reply_e01();
        }
    }
    pub fn reply_ok() {
        Self::raw_send_u8(b"$OK#9A");
        Self::flush();
    }
    pub fn reply_e01() {
        Self::raw_send_u8(b"$E01#A6");
        Self::flush();
    }
    //
    pub fn add_u32(&mut self, val: u32) {
        let mut tmp: [u8; 8] = [0; 8];

        let mut shift = 24;
        let mut offset = 0;
        for _i in 0..4 {
            let digit: u8 = ((val >> shift) & 0xff) as u8;
            crate::parsing_util::u8_to_ascii_to_buffer(digit, &mut tmp[offset..(offset + 2)]);
            shift -= 8;
            offset += 2;
        }

        self.add_u8(&tmp);
    }
    pub fn add_u32_le(&mut self, val: u32) {
        let mut tmp: [u8; 8] = [0; 8];

        let mut shift = 0;
        let mut offset = 0;
        for _i in 0..4 {
            let digit: u8 = ((val >> shift) & 0xff) as u8;
            crate::parsing_util::u8_to_ascii_to_buffer(digit, &mut tmp[offset..(offset + 2)]);
            shift += 8;
            offset += 2;
        }

        self.add_u8(&tmp);
    }
    pub fn simple_send_u32_le(val: u32) {
        let mut e = Self::new();

        e.begin();
        e.add_u32_le(val);
        e.end();
    }
    //
    pub fn begin(&mut self) {
        self.checksum = 0;
        self.escaped = false;
        Self::raw_send_u8(&[packet_symbols::CHAR_START]);
    }
    //
    pub fn hexify_and_raw_send(str: &[u8]) {
        let buffer = get_temp_buffer();
        let mut byt = str;
        while !byt.is_empty() {
            let n = core::cmp::min(byt.len(), TEMP_BUFFER_SIZE / 2);
            for i in 0..n {
                u8_to_ascii_to_buffer(byt[i], &mut buffer[2 * i..]);
            }
            Self::raw_send_u8(&buffer[..2 * n]);
            byt = &byt[n..];
        }
    }
    //
    pub fn hex_and_add(&mut self, data: &str) {
        let mut byt = data.as_bytes();
        let buffer = get_temp_buffer();
        while !byt.is_empty() {
            let n = core::cmp::min(byt.len(), TEMP_BUFFER_SIZE / 2);
            for i in 0..n {
                u8_to_ascii_to_buffer(byt[i], &mut buffer[2 * i..]);
            }
            self.add_u8(&buffer[..2 * n]);
            byt = &byt[n..];
        }
    }
    //
    pub fn add_u8(&mut self, byt: &[u8]) {
        let mut n = 0;
        let buffer = get_temp_buffer();

        for c in byt {
            match *c {
                packet_symbols::CHAR_ESCAPE2
                | packet_symbols::CHAR_ESCAPE
                | packet_symbols::CHAR_START
                | packet_symbols::CHAR_END => {
                    buffer[n] = packet_symbols::CHAR_ESCAPE;
                    buffer[n + 1] = *c ^ 0x20;
                    n += 2;
                    self.checksum += (*c ^ 0x20) as usize;
                    self.checksum += packet_symbols::CHAR_ESCAPE as usize;
                }
                _ => {
                    buffer[n] = *c;
                    n += 1;
                    self.checksum += *c as usize;
                }
            };
            if n > TEMP_BUFFER_SIZE - 2 {
                Self::raw_send_u8(&buffer[0..n]);
                n = 0;
            }
        }
        if n > 0 {
            Self::raw_send_u8(&buffer[0..n]);
        }
    }
    //
    pub fn add(&mut self, data: &str) {
        self.add_u8(data.as_bytes());
    }
    //
    pub fn end(&mut self) {
        // send end
        Self::raw_send_u8(&[packet_symbols::CHAR_END]);
        // Send checksum
        Self::raw_send_u8(&u8_to_ascii((self.checksum & 0xff) as u8));
        Self::flush();
    }
    pub fn error(code: u8) {
        let mut e = Self::new();
        let zero: char = (b'0' + code) as char;
        let mut tmp = [0u8; 4];
        e.begin();
        e.add("E");
        e.add(zero.encode_utf8(&mut tmp));
        e.end();
    }
    //
    pub fn simple_send(data: &str) {
        let mut e = Self::new();
        e.begin();
        e.add(data);
        e.end();
    }
    //
    pub fn raw_send(data: &str) {
        rngdb_send_data_u8(data.as_bytes());
    }
    //
    pub fn raw_send_u8(data: &[u8]) {
        rngdb_send_data_u8(data);
    }
    pub fn flush() {
        rngdb_output_flush();
    }
}
