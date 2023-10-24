
#[allow(unused_imports)]

#[cfg(feature = "native")]
#[macro_export]
macro_rules! bmplog
        {
            ($x:expr) => {
                if(log_enabled)
                {
                    lnLogger!($x)
                }
            };
        
            ($x:expr, $($y:expr),+) => {
                if(log_enabled)
                {
                    lnLogger!( $x, $($y),+)
                }
            };            
        }
        //--
#[cfg(feature = "native")]        
#[macro_export]
macro_rules! bmpwarning
        {
            ($x:expr) => {
                    lnLogger!($x)
            };

            ($x:expr, $($y:expr),+) => {
                    lnLogger!( $x, $($y),+)
            };            
        }       

#[cfg(feature = "native")]
#[macro_export]
macro_rules! setup_log
{
    ($x:expr) => {      
        use rnarduino::{lnLogger,lnLogger_init};     
        lnLogger_init!();
        static log_enabled : bool = $x;
       
    }
}

// ------------Hosted mode ------------
//#[cfg(feature = "hosted")]
////#[macro_export]
//-------------

#[cfg(feature = "hosted")]
#[macro_export]
macro_rules! bmplog
        {
            ($x:expr) => {
                if(log_enabled)
                {
                    print!("{}",($x))
                }
            };
        
            ($x:expr, $($y:expr),+) => {
                if(log_enabled)
                {
                    print!( $x, $($y),+)
                }
            };            
        }
        //--
#[cfg(feature = "hosted")]        
#[macro_export]
macro_rules! bmpwarning
        {
            ($x:expr) => {
                print!($x)
            };

            ($x:expr, $($y:expr),+) => {
                print!( $x, $($y),+)
            };            
        }       

#[cfg(feature = "hosted")]
#[macro_export]
macro_rules! setup_log
{
    ($x:expr) => {      
        static log_enabled : bool = $x;       
        extern crate std;
        use std::print;     
    }
}