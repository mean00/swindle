
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

use crate::util::{ascii_to_hex,u8_to_ascii,u8_to_ascii_inplace};
use crate::util::glog;
use crate::{rngdb_send_data,rngdb_send_data_u8,rngdb_output_flush};
use crate::packet_symbols;
use hex;


// DF
const TEMP_BUFFER_SIZE : usize = 64;
static mut  temp_buffer : [u8;TEMP_BUFFER_SIZE] = [0;TEMP_BUFFER_SIZE];

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
    pub fn hex_and_add(&mut self, data :  &str)
    {
        let mut byt = data.as_bytes();
        while byt.len()>0
        {
            let n=core::cmp::min(byt.len(),TEMP_BUFFER_SIZE/2);
            for i in 0..n
            {
                unsafe {  u8_to_ascii_inplace(byt[i],&mut temp_buffer[2*i..]);          }
            }
            unsafe { self.add_u8(&temp_buffer[..2*n]) };
            byt=&byt[n..];
        }
    }
    pub fn add_u8(&mut self, byt :  &[u8])
    {        
        let l=byt.len();        
        let mut n=0;

        for i in 0..l        
        {
            let c = byt[i];
            match c
            {
                packet_symbols::CHAR_ESCAPE2 | packet_symbols::CHAR_ESCAPE | packet_symbols::CHAR_START | packet_symbols::CHAR_END =>
                {
                    unsafe {
                    temp_buffer[n]=packet_symbols::CHAR_ESCAPE;
                    temp_buffer[n+1]=c^0x20;
                    }
                    n+=2;
                    self.checksum += (c^0x20) as usize;
                    self.checksum += packet_symbols::CHAR_ESCAPE as usize;
                },
             _ => {
                    unsafe {
                    temp_buffer[n]=c;
                    }
                    n+=1;
                    self.checksum += c as usize;
             },
            };
            if n> TEMP_BUFFER_SIZE-2
            {
                unsafe {
                rngdb_send_data_u8(&temp_buffer[0..n]);
                }
                n=0;
            }
        }
        if n>0
        {
            unsafe {
            rngdb_send_data_u8(&temp_buffer[0..n]);
            }
        }
    }
    pub fn add(&mut self, data :  &str)
    {
        self.add_u8(data.as_bytes());
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