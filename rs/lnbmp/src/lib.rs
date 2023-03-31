#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

mod util;
mod parsing_util;
mod packet_symbols;
mod decoder;
mod encoder;
mod commands;
mod rn_bmp_cmd_c;
mod bmp;
mod glue;


use packet_symbols::{CHAR_ACK,CHAR_NACK,INPUT_BUFFER_SIZE};
use crate::decoder::gdb_stream;
extern crate alloc;
use crate::util::{glog,glog1};
use alloc::vec::Vec;
use decoder::RESULT_AUTOMATON;
use numtoa::NumToA;


//

#[no_mangle]

static mut autoauto : Option<gdb_stream::<INPUT_BUFFER_SIZE>> = None;

#[no_mangle]
extern "C" fn rngdbstub_init()
{
    unsafe {
    if autoauto.is_some()
    {
        //panic!("notnull");
        autoauto = None;
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
//            panic!("notsome");
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
    let mut data_as_slice : &[u8] =  core::slice::from_raw_parts(d,l);
    let empty1 : [u8;0] = [0;0];
    let empty : &[u8] = &empty1;

    // The target is running, the only valid thing we are expecting is 3 or 4 (i.e. stop request)
    // i'm not sure what happens escaping-wise if we let the parser handle it
    if crate::commands::run::target_is_running()    
    {
        if data_as_slice.len() == 1
        {
            match data_as_slice[0]
            {
                3  => crate::commands::run::target_halt() ,
                _  => glog("Warning : garbage received")
            }
        }
        return;
    }
    // the target is stopped
    // we can parse the incoming commands
    match autoauto
    {
        Some(ref mut x) => 
                    {
                        while data_as_slice.len() > 0
                        {
                            let consumed : usize;
                            let state : RESULT_AUTOMATON;
                            
                            (consumed, state) =  x.parse(data_as_slice);

                            match state
                            {
                                RESULT_AUTOMATON::Ready => 
                                    {
                                        // ok we have a full string...
                                        let s = x.get_result();
                                        let command : &[u8];
                                        let args : &[u8];
                                        match crate::parsing_util::split_command(s)
                                        {
                                            None => {
                                                        crate::util::glog("Cannot convert string");
                                                        command = empty;
                                                        args = empty;                                                        
                                                    },
                                            Some( (x,y) ) =>
                                                    {
                                                        command = x;
                                                        args = y;
                                                    }
                                        }
                                        if command.len()==0
                                        {
                                            crate::util::glog("Cannot read string");                                            
                                        }
                                        else
                                        {                                            
                                            rngdb_send_data( CHAR_ACK ); 
                                            rngdb_output_flush( );
                                            let as_string = core::str::from_utf8_unchecked(command);
                                            commands::exec(as_string, args);
                                        }
                                    },
                                RESULT_AUTOMATON::Error => {rngdb_send_data( CHAR_NACK ); rngdb_output_flush( );},
                                RESULT_AUTOMATON::Continue => (),
                                RESULT_AUTOMATON::Reset => (),
                            }
                            data_as_slice=&data_as_slice[consumed..];

                        }
                    },
        None => panic!("noauto"),
    };
    }
}

// EOF
