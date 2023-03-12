
use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::parsing_util::hex_to_u8s;

use crate::encoder::encoder;
use numtoa::NumToA;
use crate::bmp;


static mut running :bool  = false;

pub enum HaltState
{
    Running     ,
    Error       ,
    Request     ,
    Stepping    ,
    Breakpoint  ,
    Watchpoint(u32)  ,
    Fault       ,
}


//
fn reply_2( prefix : &str, num : u32)
{
    let mut buffer: [u8;20] = [0; 20]; 
    let mut e = crate::encoder::encoder::new();
    e.begin();
    e.add(prefix);
    e.add(num.numtoa_str(16,&mut buffer));  
    e.end();
}
//
fn reply_4( prefix : &str, num : u32, prefix2 : &str, num2: u32)
{
    let mut buffer: [u8;20] = [0; 20]; 
    let mut e = crate::encoder::encoder::new();
    e.begin();
    e.add(prefix);
    e.add(num.numtoa_str(16,&mut buffer));  
    e.add(prefix2);
    e.add(num2.numtoa_str(16,&mut buffer));  
    e.end();
}


#[no_mangle]
extern "C" fn rngdbstub_poll()
{
    // this is called regularily
    let check: bool;
    unsafe {
     check =  bmp::bmp_attached() && running;
    }
    if check
    {
        match bmp::bmp_poll()
        {
            HaltState::Running      => return,                  // nothing to do !
            HaltState::Error        => reply_2("X", 29),    // SIGLOST
            HaltState::Request      => reply_2("T", 2),     // SIGINT
            HaltState::Watchpoint(wp)  => reply_4("T", 5, "watch:",wp as u32 ), // SIGTRAP
            HaltState::Fault        => reply_2("T", 11),  // SIGSEGV
            HaltState::Breakpoint   => reply_2("T", 5), // SIGTRAP
            _ => panic!("unsupported halt state"),
        }
        unsafe {running = false;}
    }
}

//
pub fn _R(_command : &str, _args : &Vec<&str>) -> bool
{
   encoder::reply_bool( crate::bmp::bmp_reset_target());
   true
}
pub fn _k(_command : &str, _args : &Vec<&str>) -> bool
{
    encoder::reply_bool( crate::bmp::bmp_reset_target());
    true
}
//vCont[;action[:thread-id]]…’
//
pub fn _c(command : &str, args : &Vec<&str>) -> bool
{
    _vCont("vCont",args)
}
pub fn _vCont(command : &str, _args : &Vec<&str>) -> bool
{
    
    if command.starts_with("vCont?")
    {
        let mut e = encoder::new();
        e.begin();
        e.add("vCont;c;s;t");
        e.end();
        return true;
    }
    if command.len()<7 // naked vcond
    {
        unsafe {running = true;}
        crate::bmp::bmp_halt_resume(false);
        //encoder::reply_ok();
        return true;
    }
    let command_bytes = command.as_bytes();
    return match command_bytes[6]
    {
        b'c' => 
        {
            unsafe {running = true;}
            crate::bmp::bmp_halt_resume(false);
          //  encoder::reply_ok();
            true
        },
        b's' => 
        {
            crate::bmp::bmp_halt_resume(true);
            unsafe {running = true;}
         //   encoder::reply_ok();
            true
        },
        b't' => 
        {
            //crate::bmp::bmp_halt_resume(true);
            encoder::reply_e01(); // !! TODO !!
            true
        },
        _ => false,
    };
    
}