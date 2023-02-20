//-------------------------------------------
//
//
//
//
//-------------------------------------------
use gdbstub::conn::Connection;
use gdbstub::conn::ConnectionExt;
use rnarduino as rn;
//use rn::rn_usb_cdc::rnCDC;
//use rn::rn_usb_cdc::rnCDCEventHandler;
use rn::rn_os_helper::log;
//use rnarduino::rnarduino::lnUsbCDC_lnUsbCDCEvents;
use cty::c_int;
//
extern crate alloc;
use alloc::boxed::Box;
//
pub struct cdc_connection
{
   // cdc : Box<rnCDC>,
}
//-----------------------------------
/*
impl rnCDCEventHandler for cdc_connection
{
    fn  eventHandler( &self, interface: c_int ,  event : lnUsbCDC_lnUsbCDCEvents, payload: u32)
    {
        log("Event !");
    }

}*/
//--
impl cdc_connection
{
    pub fn new() -> Result<Self, &'static str >
    {
        Ok(cdc_connection 
            {
               // cdc : rnCDC::new(0,None),
            }
        )
    }
}
//--
impl Connection for cdc_connection
{    
    type Error = &'static str;
    /// Write a single byte.
    fn write(&mut self, byte: u8) -> Result<(), Self::Error>
    {
        return Err("write");
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        for b in buf {
            self.write(*b)?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error>
    {
        return Err("flush");
    }

    fn on_session_start(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }     
}
//
impl ConnectionExt for cdc_connection
{    
   
    /// Read a single byte.
    fn read(&mut self) -> Result<u8, Self::Error>
    {
    return Err("read");
    }

    /// Peek a single byte. This MUST be a **non-blocking** operation, returning
    /// `None` if no byte is available.
    ///
    /// Returns a byte (if one is available) without removing that byte from the
    /// queue. Subsequent calls to `peek` MUST return the same byte.
    fn peek(&mut self) -> Result<Option<u8>, Self::Error>
    {
    return Err("peek");
    }
}
// EOF
