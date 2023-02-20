#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]


use gdbstub::stub::state_machine::GdbStubStateMachine;
use gdbstub::stub::MultiThreadStopReason;
use gdbstub::stub::{DisconnectReason, GdbStubBuilder, GdbStubError};

use gdbstub::conn::ConnectionExt;

use rnarduino::rn_usb::*;
use rnarduino::rn_usb::rnUSB;
use rnarduino::rn_usb_cdc::*;
use rnarduino::rn_usb_cdc::rnCDC;
use rnarduino::rn_os_helper::{log};

mod conn;
mod gdb;
mod print_str;

use crate::print_str::print_str;

//use rnarduino::rn_os_helper::{rn_create_task,rnTaskEntry,delay_ms};
use rnarduino::rn_os_helper::{delay_ms};
use conn::cdc_connection;
//
//
//
struct usb_h
{

}
impl usb_event_handler for usb_h
{
    fn  handler(  &mut self,  _event : usb_events )
    {        
        log("Got usb event ");
    }
}
struct cdc_h
{

}
impl cdc_event_handler for cdc_h
{
    fn  handler(  &mut self, _interface : usize, _event : cdc_events ,  _payload : u32)
    {        
        log("Got cdc event ");
    }
}
//
//
//
fn rust_main() -> Result<(), i32> {
    print_str("----Running example_no_std...-----\n");

    
    let mut usbh  : usb_h = usb_h {};
    let mut cdch  : cdc_h = cdc_h {};
    // init usb
    let mut usb = rnUSB::new(0, &mut usbh);
    usb.set_configuration();
    let _cdc = rnCDC::new(0, &mut cdch);
    print_str("Starting usb  \n");
    usb.start();
    //----
    loop
    {
        delay_ms(10);
    }

    let mut target = gdb::DummyTarget::new();

    let conn = match cdc_connection::new() {
        Ok(c) => c,
        Err(e) => {
            print_str("could not start TCP server:");
            print_str(e);
            return Err(-1);
        }
    };

    let mut buf = [0; 4096];
    let gdb = GdbStubBuilder::new(conn)
        .with_packet_buffer(&mut buf)
        .build()
        .map_err(|_| 1)?;

    print_str("Starting GDB session...");

    let mut gdb = gdb.run_state_machine(&mut target).map_err(|_| 1)?;

    let res = loop {
        gdb = match gdb {
            GdbStubStateMachine::Idle(mut gdb) => {
                let byte = gdb.borrow_conn().read().map_err(|_| 1)?;
                match gdb.incoming_data(&mut target, byte) {
                    Ok(gdb) => gdb,
                    Err(e) => break Err(e),
                }
            }
            GdbStubStateMachine::Running(gdb) => {
                match gdb.report_stop(&mut target, MultiThreadStopReason::DoneStep) {
                    Ok(gdb) => gdb,
                    Err(e) => break Err(e),
                }
            }
            GdbStubStateMachine::CtrlCInterrupt(gdb) => {
                match gdb.interrupt_handled(&mut target, None::<MultiThreadStopReason<u32>>) {
                    Ok(gdb) => gdb,
                    Err(e) => break Err(e),
                }
            }
            GdbStubStateMachine::Disconnected(gdb) => break Ok(gdb.get_reason()),
        }
    };

    match res {
        Ok(disconnect_reason) => match disconnect_reason {
            DisconnectReason::Disconnect => print_str("GDB Disconnected"),
            DisconnectReason::TargetExited(_) => print_str("Target exited"),
            DisconnectReason::TargetTerminated(_) => print_str("Target halted"),
            DisconnectReason::Kill => print_str("GDB sent a kill command"),
        },
        Err(GdbStubError::TargetError(_e)) => {
            print_str("Target raised a fatal error");
        }
        Err(_e) => {
            print_str("gdbstub internal error");
        }
    }

    Ok(())
}
//
//
//
#[no_mangle]
extern "C" fn rnLoop() 
{
    match rust_main()
    {
        Ok(_x) => (),
        Err(_y) => (),
    }
}

// EOF
