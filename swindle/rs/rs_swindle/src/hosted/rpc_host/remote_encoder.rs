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
//#![allow(non_upper_case_globals)]
//#![allow(non_camel_case_types)]
//#![allow(non_snake_case)]
#![allow(dead_code)]
//#![allow(unused_imports)]

use crate::SyncCell;
use crate::hosted::transport::{platform_buffer_flush, platform_buffer_write};
use crate::packet_symbols;
use crate::rpc_wire::{ESCAPE_XOR, RPC_ESCAPE};
use crate::parsing_util::{
    u8_to_ascii, u8_to_ascii_to_buffer, u32_to_ascii_be_buffer, u32_to_ascii_le_buffer,
};
//use crate::{rngdb_output_flush, rngdb_send_data, rngdb_send_data_u8};
use crate::rngdb_send_data_u8;

// DF -- not thread safe, not re-entrant,ugly but simple
const TEMP_BUFFER_SIZE: usize = 64;
static TEMP_BUFFER: SyncCell<[u8; TEMP_BUFFER_SIZE]> = SyncCell::new([0; TEMP_BUFFER_SIZE]);

setup_log!(false);
//use crate::{bmplog, bmpwarning};

pub struct rpc_encoder {
    checksum: usize,
    escaped: bool,
}

// this buffer is NOT thread safe and should be used
// only in a single shot

fn get_temp_buffer() -> &'static mut [u8] {
    unsafe { TEMP_BUFFER.as_mut() }
}
//
//
impl rpc_encoder {
    //
    pub fn new() -> Self {
        rpc_encoder {
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
    pub fn add_u32_no_padding(&mut self, val: u32) {
        let mut tmp = [0u8; 8];
        u32_to_ascii_be_buffer(val, &mut tmp);
        let leading_zero_bytes = (val.leading_zeros() / 8) as usize;
        let skip_bytes = leading_zero_bytes.min(3);
        self.add_u8(&tmp[skip_bytes * 2..8]);
    }
    pub fn add_u32_le(&mut self, val: u32) {
        let mut tmp = [0u8; 8];
        u32_to_ascii_le_buffer(val, &mut tmp);
        self.add_u8(&tmp);
    }
    pub fn add_u8_hex(&mut self, val: u32) {
        let tmp = u8_to_ascii((val & 0xff) as u8);
        self.add_u8(&tmp);
    }
    pub fn add_u16_le(&mut self, val: u32) {
        let mut tmp: [u8; 4] = [0; 4];
        let v = val as u16;
        u8_to_ascii_to_buffer((v & 0xff) as u8, &mut tmp[0..2]);
        u8_to_ascii_to_buffer(((v >> 8) & 0xff) as u8, &mut tmp[2..4]);
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
        Self::raw_send_u8(&[packet_symbols::RPC_START]);
    }
    //
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
    /// Add raw bytes to the current packet, escaping the frame markers.
    ///
    /// The payload is CBOR now, so this is the normal path: every byte that
    /// may not appear in a frame body (`!`, `#`, `$`, `{`, `}` — see
    /// [`crate::rpc_wire::must_escape`]) goes out as `}` + byte XOR `0x20`,
    /// and both bytes are summed into the checksum.
    pub fn add_u8(&mut self, byt: &[u8]) {
        let mut n = 0;
        let buffer = get_temp_buffer();

        for c in byt {
            if crate::rpc_wire::must_escape(*c) {
                buffer[n] = RPC_ESCAPE;
                buffer[n + 1] = *c ^ ESCAPE_XOR;
                n += 2;
                self.checksum += (*c ^ ESCAPE_XOR) as usize;
                self.checksum += RPC_ESCAPE as usize;
            } else {
                buffer[n] = *c;
                n += 1;
                self.checksum += *c as usize;
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
    /// A CBOR encoder writing the parameter array of this request.
    ///
    /// ```ignore
    /// let mut e = rpc_encoder::new();
    /// e.begin();
    /// e.add_u8(&[RPC_GEN_PACKET, RPC_GEN_NRST_SET]);
    /// {
    ///     let mut cbor = e.cbor();
    ///     let _ = cbor.array(1);
    ///     let _ = cbor.bool(assert);
    /// }
    /// e.end();
    /// ```
    pub fn cbor(&mut self) -> minicbor::Encoder<&mut Self> {
        minicbor::Encoder::new(self)
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
        platform_buffer_write(data);
        //rngdb_send_data_u8(data);
    }
    pub fn flush() {
        platform_buffer_flush();
    }
}

/// The CBOR encoder writes straight into the packet: every byte goes through
/// [`rpc_encoder::add_u8`], which escapes it and folds it into the checksum.
impl minicbor::encode::Write for rpc_encoder {
    type Error = core::convert::Infallible;

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        self.add_u8(buf);
        Ok(())
    }
}
