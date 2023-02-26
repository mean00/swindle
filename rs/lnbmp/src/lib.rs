#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

mod util;
mod automaton;
mod commands;

use crate::automaton::input_stream;
extern crate alloc;

//#[cfg(feature = "hosted")]
//extern crate std;
//#[cfg(feature = "hosted")]
//use std::print;

//#[cfg(feature = "native")]

use alloc::vec::Vec;
use automaton::RESULT_AUTOMATON;
//
const INPUT_BUFFER_SIZE: usize = 512;
const ACK : &str = "+";
const NACK : &str = "-";

#[no_mangle]

static mut autoauto : Option<input_stream::<INPUT_BUFFER_SIZE>> = None;

#[no_mangle]
extern "C" fn rngdbstub_init()
{
    unsafe {
    if autoauto.is_some()
    {
        panic!("notnull");
    }
    autoauto = Some(input_stream::<INPUT_BUFFER_SIZE>::new());
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
/**
 * 
 */
fn          rngdb_send_data( data : &str)
{
    ()
}
fn          rngdb_output_flush()
{
    ()
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
                                                commands::exec(tokens);
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
