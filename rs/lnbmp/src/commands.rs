
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html
use crate::util::glog;
use alloc::vec::Vec;
use crate::{rngdb_send_data,rngdb_output_flush};
use crate::encoder::encoder;


type Callback = fn(tokns : &Vec<&str>)->bool;
struct CommandTree
{
    command : &'static str,
    args    : usize,
    cb      : Callback,
}


const main_command_tree: [CommandTree;4] = 
[
    CommandTree{ command: "!",args: 0,          cb: _extendedMode },
    CommandTree{ command: "Hg",args: 0,         cb: _Hg },    
    CommandTree{ command: "vMustReply",args: 0, cb: _vMustReply },
    CommandTree{ command: "q",args: 0,          cb: _q },
];
const q_command_tree: [CommandTree;2] = 
[
    CommandTree{ command: "qSupported",args: 0, cb: _qSupported },
    CommandTree{ command: "qSupported",args: 0, cb: _qSupported },
];



fn exec_one(tree : &[CommandTree], tokns : &Vec<&str>) -> bool
{
   let command=tokns[0];    // the first item is the command
   glog(command);
   for i in 0..tree.len()
   {
        let c = &tree[i]; 
        if command.starts_with( c.command )   // if the expected command begins with the receive command..
        {
            return (c.cb)(tokns);
        }
   }    
   return false;
}


pub fn exec(tokns : &Vec<&str>)
{
    if !exec_one(&main_command_tree,tokns)
    {

    }
}
//
//
//

pub fn _q(tokns : &Vec<&str>) -> bool
{
    return exec_one(&q_command_tree,tokns);
}
//
//
fn _vMustReply(_tokns : &Vec<&str>) -> bool
{
    encoder::simple_send("");    
    return true;
}
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
fn _extendedMode(_tokns : &Vec<&str>) -> bool
{
    encoder::simple_send("OK");    
    return true;
}
// select thread
fn _Hg(_tokns : &Vec<&str>) -> bool
{
    encoder::simple_send("OK");    
    return true;
}
fn _qXferXml(_tokns : &Vec<&str>) -> bool
{

    return true;
}
