use core::convert::Infallible;
#[allow(unused_imports)]
#[cfg(feature = "native")]
use rnarduino as rn;
use ufmt::uWrite;

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
macro_rules! setup_log {
    ($x:expr) => {
        #[allow(unused)]
        use rnarduino::{lnLogger, lnLogger_init};
        lnLogger_init!();
        static log_enabled: bool = $x;
    };
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
macro_rules! setup_log {
    ($x:expr) => {
        static log_enabled: bool = $x;
        extern crate std;
        #[allow(unused_imports)]
        use std::print;
    };
}

//---------------------------------
//
//---------------------------------

pub struct G;

impl uWrite for G {
    type Error = Infallible;

    fn write_str(&mut self, s: &str) -> Result<(), Infallible> {
        crate::glue::gdb_out_rs(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! gdb_print {

    ($x:expr) => {
        uwrite!(&mut G, "{}", $x).unwrap()
    };

    ($x:expr, $($y:expr),+) => {
        uwrite!(&mut G, $x, $($y),+).unwrap()
    };
}

#[macro_export]
macro_rules! gdb_print_init {
    () => {
        #[cfg(feature = "hosted")]
        #[allow(unused)]
        use ufmt::uwrite;
        #[allow(unused)]
        use $crate::bmplogger::G;
    };
}
