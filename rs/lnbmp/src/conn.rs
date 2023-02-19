//-------------------------------------------
//
//
//
//
//-------------------------------------------
use gdbstub::conn::Connection;
use gdbstub::conn::ConnectionExt;


//
pub struct cdc_connection
{

}
//--
impl cdc_connection
{
    pub fn new() -> Result<Self, &'static str >
    {
        Ok(cdc_connection {})
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
