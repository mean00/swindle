
use crate::util::glog;
use alloc::vec::Vec;
use crate::{rngdb_send_data,rngdb_output_flush};


type Callback = fn(tokns : &Vec<&str>)->bool;
struct CommandTree
{
    command : &'static str,
    args    : usize,
    cb      : Callback,
}
const NB_COMMANDS : usize = 2;
const command_tree: [CommandTree;NB_COMMANDS] = 
[
    CommandTree{ command: "qSupported",args: 0,cb: _qSupported },
    CommandTree{ command: "vMustReply",args: 0,cb: _vMustReply },
];


pub fn exec(tokns : &Vec<&str>)
{
   let command=tokns[0];    // the first item is the command
   glog(command);
   for i in 0..NB_COMMANDS
   {
        let c = &command_tree[i]; 
        if command.starts_with( c.command )   // if the expected command begins with the receive command..
        {
            (c.cb)(tokns);
        }
   }
    
}
//
//
fn _vMustReply(tokns : &Vec<&str>) -> bool
{
    rngdb_send_data("+#\0\0");
    rngdb_output_flush();
    return true;
}
//
//
fn _qSupported(tokns : &Vec<&str>) -> bool
{
    rngdb_send_data("PacketSize=");
    rngdb_send_data("200");
    //%X
    rngdb_send_data(";qXfer:memory-map:read+;qXfer:features:read+");
    rngdb_output_flush();
    return true;
}