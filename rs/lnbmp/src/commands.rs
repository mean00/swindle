
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html
use crate::util::glog;
use alloc::vec::Vec;
use crate::encoder::encoder;

mod q;
mod v;
mod mon;


use q::_q;
use v::_v;

type Callback = fn(tokns : &Vec<&str>)->bool;

struct CommandTree
{
    command : &'static str,
    args    : usize,
    cb      : Callback,
}


const main_command_tree: [CommandTree;7] = 
[
    CommandTree{ command: "!",args: 0,          cb: _extendedMode },// enable extended mode
    CommandTree{ command: "Hg",args: 0,         cb: _Hg },          // select thread
    CommandTree{ command: "Hc",args: 0,         cb: _Hc },          // 
    CommandTree{ command: "v",args: 0,          cb: _v },  // test
    CommandTree{ command: "q",args: 0,          cb: _q },           // see q commands in commands/q.rs
    CommandTree{ command: "g",args: 0,          cb: _g },           // read registers
    CommandTree{ command: "?",args: 0,          cb: _mark },        // reason for halt
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
        {
            encoder::simple_send("");            // unsupported
        }        
    }
}
//
//
fn _extendedMode(_tokns : &Vec<&str>) -> bool
{
    encoder::simple_send("OK");    
    true
}
// select thread
fn _Hg(_tokns : &Vec<&str>) -> bool
{
    encoder::simple_send("OK");    
    true
}
// select thread
fn _Hc(_tokns : &Vec<&str>) -> bool
{
    encoder::simple_send("OK");    
    true
}

// Read registers
fn _g(_tokns : &Vec<&str>) -> bool
{    
    //NOTARGET
    encoder::simple_send("EFF");
    true
}
//
// Request reason for halt
fn _mark(_tokns : &Vec<&str>) -> bool
{
    //NOTARGET
    encoder::simple_send("W00");
    true
}

// EOF