

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![macro_use]
#![macro_escape]

// ------------Native mode ------------
#[cfg(feature = "native")]
#[macro_export]
macro_rules! setup_log
{
    ($enabled :tt ) => {
        use rnarduino::rn_os_helper::{log,log1};
        use ufmt::uDisplay;
        use crate::util::{glog,glog1,glogx};
        fn bmplog(  s: &str)
        {
            if $enabled
            {
                glog(s);
            }
        }
        fn bmpwarning<T: uDisplay> (s : &str, v: T)
        {
            glog("Warning:");
            glog1(s,v);
        }

        fn bmplog1<T: uDisplay> (s : &str, v: T)
        {
            if $enabled
            {
                glog1(s,v);
            }
        }
        fn bmplogx (s : &str, v: u32)
        {
            if $enabled
            {
                glog1(s,v);
            }
        }

    };
}
// ------------Hosted mode ------------
#[cfg(feature = "hosted")]
#[macro_export]
macro_rules! setup_log
{
    ($enabled : tt) => {
        extern crate std;
        use std::print;     
        

        fn bmplog(  s: &str)
        {
            if $enabled
            {
                crate::bmp::bmplog(s);
            }
        }
        fn bmpwarning<T: core::fmt::Display> (s : &str, v: T)
        {
                print!("<<{:?}:{}\n",s,v);
        }
        fn bmplog1<T: core::fmt::Display> (s : &str, v: T)
        {
            if $enabled
            {
                print!("<<{:?}:{}\n",s,v);
            }
        }
        fn bmplogx(s : &str, v: u32)
        {
            if $enabled
            {
                print!("<<{:?}:0x{:#x}\n",s,v);
            }
        }
    };
}
//-------------
