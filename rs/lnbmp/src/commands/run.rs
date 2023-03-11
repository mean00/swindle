
use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::parsing_util::hex_to_u8s;

use crate::encoder::encoder;




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
        crate::bmp::bmp_halt_resume(false);
        //encoder::reply_ok();
        return true;
    }
    let command_bytes = command.as_bytes();
    return match command_bytes[6]
    {
        b'c' => 
        {
            crate::bmp::bmp_halt_resume(false);
          //  encoder::reply_ok();
            true
        },
        b's' => 
        {
            crate::bmp::bmp_halt_resume(true);
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