

use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::bmp;
use crate::encoder::encoder;
use crate::util::hex_to_u8s;
use crate::commands::{CallbackType,exec_one,CommandTree};
//
//
//
const mon_command_tree: [CommandTree;1] = 
[
    CommandTree{ command: "swdp_scan",args: 0,      require_connected: false ,cb: CallbackType::text( _swdp_scan) },      // 
];

//
// Execute command
//
pub fn _qRcmd(command : &str, _args : &Vec<&str>) -> bool
{    
    let largs : Vec <&str>= command.split(",").collect();
    //NOTARGET
    let ln = largs.len();
    if ln !=2
    {
        return false;
    }
    // The command is hex encoded, decode it
    let mut out : [u8;32] = [0;32];    
    let rcmd = match hex_to_u8s(largs[1],&mut out)
    {
        Ok(x)    =>     x    ,
        Err(_y)    => {return false;},
    };
    // split command and args
    let command : &[u8];
    let args : &[u8];
    match crate::util::split_command(rcmd)
    {
        None => {
                    crate::util::glog("Cannot convert string (rcmd)");
                    return false;                                                       
                },
        Some( (x,y) ) =>
                {
                    command = x;
                    args = y;
                }
    }
    let as_string = unsafe {core::str::from_utf8_unchecked(command)};
    exec_one(&mon_command_tree,as_string, args)
    
}

pub fn _swdp_scan(_command : &str, _args : &Vec<&str>) -> bool
{
    glog("swdp_scan");
    if bmp::swdp_scan()
    {
        glog("success!");
        encoder::reply_ok();
        return true;
    }
    glog("fail!");
    return false;
    
}