#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
use rnarduino::rn_usb::*;
use rnarduino::rn_usb::rnUSB;
use rnarduino::rn_usb_cdc::*;
use rnarduino::rn_usb_cdc::rnCDC;
use rnarduino::rn_fast_event_group::rnFastEventGroup;
//use rnarduino::rn_os_helper::{rn_create_task,rnTaskEntry,delay_ms};
use rnarduino::rn_os_helper::{delay_ms,log};

extern crate alloc;



//
//
//
#[no_mangle]
extern "C" fn rngdbstub_init() -> bool
{
      return true;
}
extern "C" fn rngdbstub_shutdown() 
{
    /*
    unsafe {
    let mut old = wrapper.gdbstub;
    wrapper.gdbstub = None;
    wrapper.target  = None;
    }*/
}
fn run_gdb2( )  -> Result<(), i32>
{
    return Ok(())

}


#[no_mangle]
extern "C" fn rngdbstub_run( c : u8) 
{
    run_gdb2();
}

// EOF
