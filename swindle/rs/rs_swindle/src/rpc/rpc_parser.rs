use crate::bmplogger::*;
use crate::parsing_util::ascii_hex_string_to_u8s;
use crate::parsing_util::ascii_octet_to_hex;
use crate::util::xmin;
crate::setup_log!(false);
use crate::{bmplog, bmpwarning};
/**
 */
pub struct rpc_parameter_parser<'a> {
    data: &'a [u8],
}
/**
 *
 */
impl<'a> rpc_parameter_parser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        rpc_parameter_parser { data }
    }
    pub fn next_u32(&mut self) -> u32 {
        let n = xmin(self.data.len(), 8);
        let out: u32 = crate::parsing_util::u8s_string_to_u32_le(&self.data[0..n]);
        self.data = &self.data[n..];
        out
    }
    pub fn next_u32_be(&mut self) -> u32 {
        let n = xmin(self.data.len(), 8);
        let out: u32 = crate::parsing_util::u8s_string_to_u32(&self.data[0..n]);
        self.data = &self.data[n..];
        out
    }
    pub fn next_u8(&mut self) -> u32 {
        let left: u32 = ascii_octet_to_hex(self.data[0], self.data[1]) as u32;
        self.data = &self.data[2..];
        left
    }
    pub fn next_u16_be(&mut self) -> u32 {
        let left = self.next_u8();
        let right = self.next_u8();
        (left << 8) + (right << 0)
    }

    pub fn next_u16(&mut self) -> u32 {
        let left = self.next_u8();
        let right = self.next_u8();
        (left << 0) + (right << 8)
    }
    pub fn next_cmd(&mut self) -> u8 {
        let cmd: u8 = self.data[0];
        self.data = &self.data[1..];
        cmd
    }
    pub fn end(&mut self) -> &[u8] {
        self.data
    }
}
// -- EOF
