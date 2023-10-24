
#[allow(unused_imports)]
#[macro_export]
macro_rules! bmplog
        {
            ($x:expr) => {
                lnLogger!($x)
            };
        
            ($x:expr, $($y:expr),+) => {
                lnLogger!( $x, $($y),+)
            };            
        }
        //--
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

//#[cfg(feature = "native")]
#[macro_export]
macro_rules! setup_log
{
    ($x:expr) => {      
        use rnarduino::{lnLogger,lnLogger_init};     
        lnLogger_init!();
       
    }
}

// ------------Hosted mode ------------
//#[cfg(feature = "hosted")]
////#[macro_export]
//-------------
