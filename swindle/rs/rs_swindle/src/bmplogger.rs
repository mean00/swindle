use core::convert::Infallible;
#[allow(unused_imports)]
#[cfg(feature = "native")]
use rust_esprit as rn;
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
        use rust_esprit::{lnLogger, lnLogger_init};
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

use ufmt::uwrite;
pub struct G;

impl uWrite for G {
    type Error = Infallible;

    fn write_str(&mut self, s: &str) -> Result<(), Infallible> {
        crate::glue::gdb_out_rs(s);
        Ok(())
    }
}
impl G {
    #[inline(never)]
    pub fn print_str(s: &str) {
        crate::glue::gdb_out_rs(s);
    }

    #[inline(never)]
    pub fn print_int(val: i64) {
        let _ = uwrite!(&mut G, "{}", val);
    }
    #[inline(never)]
    pub fn print_hex(mut n: u32) {
        Self::print_str("0x");
        if n == 0 {
            Self::print_str("0");
            return;
        }
        let mut buf = [0u8; 8];
        let mut i = 7;
        const HEX_CHARS: &[u8] = b"0123456789abcdef";
        while n > 0 {
            buf[i] = HEX_CHARS[(n & 0xf) as usize];
            n >>= 4;
            if n == 0 {
                break;
            }
            i -= 1;
        }
        unsafe {
            Self::print_str(core::str::from_utf8_unchecked(&buf[i..]));
        }
    }
}
pub struct Hex<T>(pub T);
pub trait ToLog {
    fn log(self);
}

// Handle strings and double references (from iterators)
impl ToLog for &str {
    #[inline(always)]
    fn log(self) {
        G::print_str(self);
    }
}
impl ToLog for &&str {
    #[inline(always)]
    fn log(self) {
        G::print_str(self);
    }
}
// Handle characters
impl ToLog for char {
    #[inline(always)]
    fn log(self) {
        let mut buf = [0u8; 4];
        G::print_str(self.encode_utf8(&mut buf));
    }
}
// Handle Booleans
impl ToLog for bool {
    #[inline(always)]
    fn log(self) {
        G::print_str(if self { "true" } else { "false" });
    }
}

// Handle all integer types
impl ToLog for i32 {
    #[inline(always)]
    fn log(self) {
        G::print_int(self as i64);
    }
}
impl ToLog for u32 {
    #[inline(always)]
    fn log(self) {
        G::print_int(self as i64);
    }
}
impl ToLog for i16 {
    #[inline(always)]
    fn log(self) {
        G::print_int(self as i64);
    }
}
impl ToLog for u16 {
    #[inline(always)]
    fn log(self) {
        G::print_int(self as i64);
    }
}
impl ToLog for i8 {
    #[inline(always)]
    fn log(self) {
        G::print_int(self as i64);
    }
}
impl ToLog for u8 {
    #[inline(always)]
    fn log(self) {
        G::print_int(self as i64);
    }
}
impl ToLog for i64 {
    #[inline(always)]
    fn log(self) {
        G::print_int(self);
    }
}
impl ToLog for u64 {
    #[inline(always)]
    fn log(self) {
        G::print_int(self as i64);
    }
}
impl ToLog for usize {
    #[inline(always)]
    fn log(self) {
        G::print_int(self as i64);
    }
}

#[macro_export]
macro_rules! gdb_print2 {
    // --- Recursive Thin Branches ---

    // Case: Literal + Hex Wrapper
    ($label:literal, $crate::bmplogger::Hex($val:expr), $($rest:tt)*) => {
        {
            $crate::bmplogger::G::print_str($label);
            $crate::bmplogger::G::print_hex($val as u32);
            $crate::gdb_print!($($rest)*);
        }
    };

    // Case: Literal + Normal Value
    ($label:literal, $val:expr, $($rest:tt)*) => {
        {
            $crate::bmplogger::G::print_str($label);
            $crate::bmplogger::ToLog::log($val);
            $crate::gdb_print!($($rest)*);
        }
    };

    // Case: Two values side-by-side
    ($v1:expr, $($rest:tt)+) => {
        {
            $crate::bmplogger::ToLog::log($v1);
            $crate::gdb_print!($($rest)+);
        }
    };

    // --- Termination Base Cases ---
    ($crate::bmplogger::Hex($val:expr)) => { $crate::bmplogger::G::print_hex($val as u32); };
    ($label:literal) => { $crate::bmplogger::G::print_str($label); };
    ($val:expr)    => { $crate::bmplogger::ToLog::log($val); };

    // --- Fallback (FAT) ---
    ($x:expr, $($y:expr),+) => {
        {
            // core::compile_error!("Fat log detected!");
            #[allow(unused)]
            use ufmt::uWrite;
            let _ = ufmt::uwrite!(&mut $crate::bmplogger::G, $x, $($y),+);
        }
    };

}
#[macro_export]
macro_rules! gdb_print {
    // --- 1. RECURSIVE HEX MATCHES (Highest Priority) ---
    // We match the pattern "Hex(expr)" explicitly before general expressions.
    ($label:literal, Hex($val:expr), $($rest:tt)+) => {
        {
            $crate::bmplogger::G::print_str($label);
            $crate::bmplogger::G::print_hex($val as u32);
            $crate::gdb_print!($($rest)+);
        }
    };

    // --- 2. RECURSIVE GENERAL MATCHES ---
    ($label:literal, $val:expr, $($rest:tt)+) => {
        {
            $crate::bmplogger::G::print_str($label);
            $crate::bmplogger::ToLog::log($val);
            $crate::gdb_print!($($rest)+);
        }
    };

    ($val:expr, $($rest:tt)+) => {
        {
            $crate::bmplogger::ToLog::log($val);
            $crate::gdb_print!($($rest)+);
        }
    };

    // --- 3. TERMINATION BASE CASES ---
    // Look for Hex first!
    (Hex($val:expr)) => {
        $crate::bmplogger::G::print_hex($val as u32);
    };

    ($label:literal) => {
        $crate::bmplogger::G::print_str($label);
    };

    // This is line 269 in your error log. It must stay at the bottom
    // of the termination cases so it doesn't "steal" the Hex match.
    ($val:expr) => {
        $crate::bmplogger::ToLog::log($val);
    };

    // --- 4. FALLBACK ---
    ($x:expr, $($y:expr),+) => {
        {
            // core::compile_error!("Fat log detected!");
            #[allow(unused)]
            use ufmt::uWrite;
            let _ = ufmt::uwrite!(&mut $crate::bmplogger::G, $x, $($y),+);
        }
    };
}
#[macro_export]
macro_rules! gdb_println {
    ($($arg:tt)*) => {
        {
            $crate::gdb_print!($($arg)*);
            $crate::bmplogger::G::print_str("\n");
        }
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
        #[allow(unused_macros)]
        #[allow(unused)]
        use $crate::bmplogger::Hex;
        #[allow(unused)]
        use $crate::bmplogger::ToLog;
    };
}
