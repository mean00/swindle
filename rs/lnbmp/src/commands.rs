
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html
use crate::util::glog;
use alloc::vec::Vec;
use crate::encoder::encoder;

mod q;
mod v;
mod mon;
mod x;
mod registers;
mod memory;


use q::_q;
use v::_v;
use x::_X;
use memory::_m;

type Callback = fn(tokns : &Vec<&str>)->bool;

struct CommandTree
{
    command : &'static str,
    args    : usize,
    require_connected : bool,
    cb      : Callback,
}


const main_command_tree: [CommandTree;9] = 
[
    CommandTree{ command: "!",args: 0,  require_connected: false,        cb: _extendedMode },// enable extended mode
    CommandTree{ command: "Hg",args: 0, require_connected: false,        cb: _Hg },          // select thread
    CommandTree{ command: "Hc",args: 0, require_connected: false,        cb: _Hc },          // 
    CommandTree{ command: "v",args: 0,  require_connected: false,        cb: _v },  // test
    CommandTree{ command: "q",args: 0,  require_connected: false,        cb: _q },           // see q commands in commands/q.rs
    CommandTree{ command: "g",args: 0,  require_connected: false,        cb: _g },           // read registers
    CommandTree{ command: "?",args: 0,  require_connected: false,        cb: _mark },        // reason for halt
    CommandTree{ command: "X",args: 0,  require_connected: true,         cb: _X },        // write binary    
    CommandTree{ command: "m",args: 0,  require_connected: true,         cb: _m },        // read memory
];



fn exec_one(tree : &[CommandTree], tokns : &Vec<&str>) -> bool
{
   let connected : bool = crate::bmp::bmp_attached();
   let command=tokns[0];    // the first item is the command
   glog(command);
   for i in 0..tree.len()
   {
        let c = &tree[i]; 
        if command.starts_with( c.command )   // if the expected command begins with the receive command..
        {
            if !connected && c.require_connected
            {
                glog("Command not ok while not connected");
                glog(c.command);
                encoder::reply_e01();
                return true;                
            }else
            {
                return (c.cb)(tokns);
            }
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
    encoder::reply_ok();   
    true
}
// select thread
fn _Hg(_tokns : &Vec<&str>) -> bool
{
    encoder::reply_ok();
    true
}
// select thread
fn _Hc(_tokns : &Vec<&str>) -> bool
{
    encoder::reply_ok();   
    true
}

// Read registers
fn _g(_tokns : &Vec<&str>) -> bool
{       
    // this one may be called while we are not connected
    if !crate::bmp::bmp_attached()
    {
           encoder::reply_e01();
           return true;
    }
    

    let regs = crate::bmp::bmp_read_registers();
    let mut e = encoder::new();
    e.begin();
    let mut buffer : [u8;8]=[0;8];
    let n: usize = regs.len();
    for i in 0..n
    {       
        let mut reg = regs[i];
        // LE first
        for j in 0..4
        {
            crate::util::u8_to_ascii_to_buffer((reg &0xff) as u8 ,&mut buffer[2*j..]); 
            reg = reg >> 8;
        }
        e.add_u8(&buffer);
    }
    e.end();
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

