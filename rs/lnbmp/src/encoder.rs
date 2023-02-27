
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

use crate::util::{ascii_to_hex,u8_to_ascii};
use crate::util::glog;
use crate::{rngdb_send_data,rngdb_send_data_u8,rngdb_output_flush};
use crate::packet_symbols;


pub struct encoder
{
    checksum : usize,
    escaped  : bool,
}

impl encoder
{
    pub fn new() -> Self
    {
        encoder
        {
            checksum : 0,
            escaped  : false,
        }
    }
    pub fn begin(&mut self)
    {
        self.checksum = 0;
        self.escaped = false;
        rngdb_send_data_u8(&[packet_symbols::CHAR_START]);
    }
    pub fn add(&mut self, data :  &str)
    {
        let byt = data.as_bytes();
        // 
        for i in 0..byt.len()
        {
            let c= byt[i] as usize;
            self.checksum+=c;
        }
        rngdb_send_data_u8(byt);
    }
    pub fn end(&mut self)
    {
        // send end
        rngdb_send_data_u8(&[packet_symbols::CHAR_END]);
        // Send checksum

        rngdb_send_data_u8(&u8_to_ascii((self.checksum & 0xff) as u8));
        rngdb_output_flush();
    }
    pub fn simple_send(data : &str)
    {
        let mut e = Self::new();
        e.begin();
        e.add(data);
        e.end();
    }
    pub fn raw_send(data : &str)
    {
        rngdb_send_data_u8(data.as_bytes());
    }
    pub fn raw_send_u8(data : &[u8])
    {
        rngdb_send_data_u8(data);
    }

}