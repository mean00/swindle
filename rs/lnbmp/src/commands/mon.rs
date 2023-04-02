

use alloc::vec;
use alloc::vec::Vec;
use crate::util::{glog,glog1};
use crate::bmp;
use crate::encoder::encoder;
use crate::parsing_util::ascii_hex_string_to_u8s;
use crate::commands::{CallbackType,exec_one,CommandTree};
use crate::glue::gdb_out_rs;
use numtoa::NumToA;
//
//
//
const mon_command_tree: [CommandTree;3] = 
[
    CommandTree{ command: "help",args: 0,      require_connected: false ,cb: CallbackType::text( _mon_help) },      // 
    CommandTree{ command: "swdp_scan",args: 0,      require_connected: false ,cb: CallbackType::text( _swdp_scan) },      // 
    CommandTree{ command: "voltage",args: 0,      require_connected: false ,cb: CallbackType::text( _voltage) },      // 
];

//
pub fn _mon_help(_command : &str, _args : &Vec<&str>) -> bool
{
    gdb_out_rs( "mon commands :\n");
    for i in 0..mon_command_tree.len()
    {
        gdb_out_rs( "   ");
        gdb_out_rs( &(mon_command_tree[i].command));
        gdb_out_rs( ":\n");
    }
    encoder::reply_ok();
    return true;
}

//
pub fn _voltage(_command : &str, _args : &Vec<&str>) -> bool
{

    let voltage = bmp::bmp_get_target_voltage();
    let voltage32 : u32 = (voltage * 1000.) as u32;
    let mut buffer: [u8;20] = [0; 20]; 
    
    gdb_out_rs(&"Voltage (mv):");
    gdb_out_rs(voltage32.numtoa_str(10,&mut buffer));
    gdb_out_rs(&"\n");
    encoder::reply_ok();
    return true;
}

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
    let rcmd = match ascii_hex_string_to_u8s(largs[1],&mut out)
    {
        Ok(x)    =>     x    ,
        Err(_y)    => {return false;},
    };
    // split command and args
    let command : &[u8];
    let args : &[u8];
    match crate::parsing_util::split_command(rcmd)
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
/*
    Detect stuff connected to the SWD interface
    Try to use the fastest speed
 */
pub fn _swdp_scan(_command : &str, _args : &Vec<&str>) -> bool
{
    glog("swdp_scan:\n");
    let mut pivot = 4;
    let mut inc = 4;
    // is there anything at all ?
    bmp::bmp_set_wait_state(8); // starts slow..
    if !bmp::swdp_scan()
    {
        glog("fail ws=8!\n");
        return false;     // nope
    }

    loop
    {        
        glog1("swdp_scan: pivot",pivot);
        glog("\n");
        glog1("swdp_scan: inc",inc);
        glog("\n");
        bmp::bmp_set_wait_state(pivot);
        if !bmp::swdp_scan()
        {
            pivot+=inc;        
        } 
        else
        {
            pivot-=inc;
        }   
        inc=inc>>1;
        if inc==0 || pivot==0
        {
            break;
        }
    }
    // final check
    bmp::bmp_set_wait_state(pivot);
    if !bmp::swdp_scan()
    {
        glog("fail!\n");
        return false;
    }
    crate::glue::gdb_out_rs_u32("Using ", pivot) ;
    crate::glue::gdb_out_rs(" wait state\n");    
    encoder::reply_ok();
    return true;
    
}