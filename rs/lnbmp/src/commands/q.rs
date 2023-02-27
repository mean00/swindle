
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::util::hex_to_u8s;

use crate::encoder::encoder;
use super::{CommandTree,exec_one};

use super::mon::_swdp_scan;

const q_command_tree: [CommandTree;9] = 
[
    CommandTree{ command: "qSupported",args: 0, cb: _qSupported },      // supported features
    CommandTree{ command: "qXfer",args: 0,      cb: _qXfer },           // read memory map
    CommandTree{ command: "qTStatus",args: 0,   cb: _qTStatus },        // trace status
    CommandTree{ command: "qRcmd",args: 0,      cb: _qRcmd },           // execute command
    CommandTree{ command: "qAttached",args: 0,  cb: _qAttached },       // remote thread
    CommandTree{ command: "qfThreadInfo",args: 0,cb: _qfThreadInfo },   // thread info begin
    CommandTree{ command: "qsThreadInfo",args: 0,cb: _qsThreadInfo },   // List threads
    CommandTree{ command: "qC",args: 0,         cb: _qC },             // Current thread Id
    CommandTree{ command: "qOffsets",args: 0,   cb: _qOffsets },        // set code/data/.. offset
    
    
];


const mon_command_tree: [CommandTree;1] = 
[
    CommandTree{ command: "swdp_scan",args: 0, cb: _swdp_scan },      // 
];
//
//
//

pub fn _q(tokns : &Vec<&str>) -> bool
{
    return exec_one(&q_command_tree,tokns);
}
//
//
//
fn _qSupported(_tokns : &Vec<&str>) -> bool
{
    let mut e = encoder::new();
    e.begin();
    e.add("PacketSize=");
    e.add("200");    
    e.add(";qXfer:memory-map:read+;qXfer:features:read+");
    e.end();
    return true;
}
//
// Read memory map
//
fn _qXfer(_tokns : &Vec<&str>) -> bool
{
    encoder::simple_send("E01");    
    return true;
}
//
// Trace
//
fn _qTStatus(_tokns : &Vec<&str>) -> bool
{    
    return false;
}
//
// Execute command
//
fn _qRcmd(_tokns : &Vec<&str>) -> bool
{    
    let args : Vec <&str>= _tokns[0].split(",").collect();
    //NOTARGET
    let ln = args.len();
    if ln !=2
    {
        return false;
    }
    // The command is hex encoded, decode it
    let mut out : [u8;16] = [0;16];
    return match hex_to_u8s(args[1],&mut out)
    {
        Ok(x)    =>  {
                        let mut v : Vec<&str>= Vec::new();
                        unsafe {
                        v.push( core::str::from_utf8_unchecked(x));
                        }
                        exec_one(&mon_command_tree,&v)
                    },
        Err(_y)  => false,
    };
    
}
//
// Execute command
//
fn _qAttached(_tokns : &Vec<&str>) -> bool
{    
    encoder::simple_send("1");    
    return true;
}
//
//
fn _qfThreadInfo(_tokns : &Vec<&str>) -> bool
{    
    encoder::simple_send("m1");    
    return true;
}
//
//
fn _qsThreadInfo(_tokns : &Vec<&str>) -> bool
{    
    encoder::simple_send("1");    
    return true;
}
// get current thread I
fn _qC(_tokns : &Vec<&str>) -> bool
{    
    encoder::simple_send("QC1");    
    return true;
}
// get offset
fn _qOffsets(_tokns : &Vec<&str>) -> bool
{    
    encoder::simple_send("Text=0;Data=0;Bss=0");    
    return true;
}




// EOF