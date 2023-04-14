
use crate::encoder::encoder;
use core::ffi::CStr;
use numtoa::NumToA;
// Send data to the host...
#[no_mangle]
pub extern "C" fn gdb_out(fmt : *const i8) 
{    
    let mut e = encoder::new();
    e.begin();
    e.add("O");
    
    let slice : &str;
    unsafe {
            slice = match CStr::from_ptr(fmt).to_str()
            {
                Ok(x) => x,
                Err(_y) => return,
            };
    }
    e.hex_and_add(slice);    
    e.end(); 
}

pub fn gdb_out_rs(fmt : &str) 
{    
    let mut e = encoder::new();
    e.begin();
    e.add("O");    
    e.hex_and_add(fmt);    
    e.end(); 
}
// send string str + value to gdb
pub fn gdb_out_rs_u32(fmt : &str, value : u32) 
{    
    let mut buffer: [u8;12] = [0; 12]; 
    let mut e = encoder::new();
    e.begin();
    e.add("O");    
    e.hex_and_add(fmt);   
    e.hex_and_add(value.numtoa_str(10,&mut buffer));      
    e.end(); 
}


