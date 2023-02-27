#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

mod util;
mod packet_symbols;
mod decoder;
mod encoder;
mod commands;

use crate::decoder::gdb_stream;
extern crate alloc;

use alloc::vec::Vec;
use decoder::RESULT_AUTOMATON;
//
const INPUT_BUFFER_SIZE: usize = 512;
const ACK : &str = "+";
const NACK : &str = "-";

#[no_mangle]

static mut autoauto : Option<gdb_stream::<INPUT_BUFFER_SIZE>> = None;

#[no_mangle]
extern "C" fn rngdbstub_init()
{
    unsafe {
    if autoauto.is_some()
    {
        panic!("notnull");
    }
    autoauto = Some(gdb_stream::<INPUT_BUFFER_SIZE>::new());
    }
}
#[no_mangle]
extern "C" fn rngdbstub_shutdown() 
{
    unsafe {
        if autoauto.is_none()
        {
            panic!("notsome");
        }
        autoauto = None;
    }
}
/*
 * 
 */
extern "C" 
{
    fn          rngdb_send_data_c( sz : u32, ptr : *const cty::c_uchar);
    fn          rngdb_output_flush_c();
}
/*
 */
fn          rngdb_output_flush()
{
    unsafe {
        rngdb_output_flush_c();
        }
}
/*
 */
fn          rngdb_send_data( data : &str)
{
    unsafe {
    rngdb_send_data_c(data.len() as u32, data.as_ptr() );
    }
}
/*
 */
fn          rngdb_send_data_u8( data : &[u8])
{
    unsafe {
    rngdb_send_data_c(data.len() as u32, data.as_ptr() );
    }
}


/**
 * 
 * 
 */
#[no_mangle]
extern "C" fn rngdbstub_run(l : usize, d : *const cty::c_uchar ) 
{
    unsafe {
    let data_as_slice : &[u8] = core::slice::from_raw_parts(d,l);
    match autoauto
    {
        Some(ref mut x) => 
                    {
                        let mut l = l;
                        while l> 0
                        {
                            let consumed : usize;
                            let state : RESULT_AUTOMATON;
                            let empty : &str = "";
                            (consumed, state) =  x.parse(data_as_slice);
                            match state
                            {
                                RESULT_AUTOMATON::Ready => 
                                    {
                                        // ok we have a full string...
                                        let s = x.get_result();
                                        let flat_string  = match core::str::from_utf8(s)
                                        {
                                            Ok(x)       => x,
                                            Err(_y) => empty,
                                        };
                                        if flat_string.len()!=0
                                        {
                                            let tokens : Vec <&str>= flat_string.split_whitespace().collect();
                                            if tokens.len()!=0
                                            {
                                                rngdb_send_data(ACK ); 
                                                rngdb_output_flush( );
                                                commands::exec(&tokens);
                                            }
                                        }
                                    },
                                RESULT_AUTOMATON::Error => {rngdb_send_data(NACK ); rngdb_output_flush( );},
                                RESULT_AUTOMATON::Continue => (),
                                RESULT_AUTOMATON::Reset => (),
                            }
                            l-=consumed;
                        }
                    },
        None => panic!("noauto"),
    };
    }
}

// EOF
