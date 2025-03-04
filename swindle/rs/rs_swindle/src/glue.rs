use crate::encoder::encoder;
use core::ffi::CStr;
//
#[cfg(feature = "hosted")]
type my_c_str = *const i8;
#[cfg(not(feature = "hosted"))]
type my_c_str = *const u8;

// Send data to the host...
#[unsafe(no_mangle)]
pub extern "C" fn gdb_out(fmt: *const u8) {
    let mut e = encoder::new();
    e.begin();
    e.add("O");

    let slice: &str;
    unsafe {
        slice = match CStr::from_ptr(fmt as my_c_str).to_str() {
            Ok(x) => x,
            Err(_y) => return,
        };
    }
    e.hex_and_add(slice);
    e.end();
}

pub fn gdb_out_rs(fmt: &str) {
    let mut e = encoder::new();
    e.begin();
    e.add("O");
    e.hex_and_add(fmt);
    e.end();
}
