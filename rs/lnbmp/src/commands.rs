
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
mod flash;
pub mod breakpoints;
mod run;


use q::_q;
use flash::_flashv;
use v::_v;
use x::_X;
use memory::_m;
use registers::_P;
use breakpoints::_z;
use breakpoints::_Z;

use run::{_c,_R,_vCont,_k};

type Callback_raw  = fn(command : &str, args : &[u8] )  ->bool;
type Callback_text = fn(command : &str, args : &Vec<&str> )->bool;


enum CallbackType
{
    text(Callback_text),
    raw( Callback_raw),
}

struct CommandTree
{
    command             : &'static str,
    args                : usize,
    require_connected   : bool,
    cb                  : CallbackType,         // string + strings    
}


const main_command_tree: [CommandTree;17] = 
[
    CommandTree{ command: "!", args:    0, require_connected: false,   cb: CallbackType::text(_extendedMode) },// enable extended mode
    CommandTree{ command: "Hg",args:    0, require_connected: false,   cb: CallbackType::text(_Hg)      },          // select thread
    CommandTree{ command: "Hc",args:    0, require_connected: false,   cb: CallbackType::text(_Hc)       },          // 
    CommandTree{ command: "vCont",args: 0, require_connected: true,    cb: CallbackType::text(_vCont)       },        
    CommandTree{ command: "vFlash",args:0, require_connected: true,    cb: CallbackType::raw(_flashv),  },  // test
    CommandTree{ command: "v",args:     0, require_connected: false,   cb: CallbackType::raw(_v)       },  // test
    CommandTree{ command: "q",args:     0, require_connected: false,   cb: CallbackType::raw(_q)      },           // see q commands in commands/q.rs
    CommandTree{ command: "g",args:     0, require_connected: false,   cb: CallbackType::text(_g)      },           // read registers
    CommandTree{ command: "?",args:     0, require_connected: false,   cb: CallbackType::text(_mark)  },        // reason for halt
    CommandTree{ command: "X",args:     0, require_connected: true,    cb: CallbackType::text(_X)      },        // write binary    
    CommandTree{ command: "m",args:     0, require_connected: true,    cb: CallbackType::text(_m )       },        // read memory
    CommandTree{ command: "P",args:     0, require_connected: true,    cb: CallbackType::text(_P )       },    
    CommandTree{ command: "z",args:     0, require_connected: true,    cb: CallbackType::text(_z )       },        // read memory
    CommandTree{ command: "Z",args:     0, require_connected: true,    cb: CallbackType::text(_Z )       },    
    CommandTree{ command: "R",args:     0, require_connected: true,    cb: CallbackType::text(_R )       },    
    CommandTree{ command: "k",args:     0, require_connected: true,    cb: CallbackType::text(_k )       },    
    CommandTree{ command: "c",args:     0, require_connected: true,    cb: CallbackType::text(_c )       },    

];



fn exec_one(tree : &[CommandTree], command : &str, args : &[u8]) -> bool
{
   let connected : bool = crate::bmp::bmp_attached();   
   glog(command);
   let empty : &str = "";
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
                // Is it a regular callback or binary callback
                return match c.cb
                {
                    CallbackType::text(y) =>
                    {
                            // split args by ":"  
                            let as_string : &str = match core::str::from_utf8(args)
                            {
                                Ok(x) => x,
                                Err(_x)    => empty,
                            };
                            let conf : Vec <&str>= as_string.split(':').collect();
                            (y)(command, &conf)
                    },
                    CallbackType::raw(x) => (x)(command, args),
                };
               // return (c.cb)(command, args);
            }
        }
   }    
   return false;
}


pub fn exec(command : &str,  args : &[u8]) 
{  
    if !exec_one(&main_command_tree,command, args)
    {
        {
            encoder::simple_send("");            // unsupported
            crate::util::glog1("Unsupported cmd :",command);
        }        
    }
}
//
//
fn _extendedMode(_command : &str, _args : &Vec<&str>) -> bool
{
    encoder::reply_ok();   
    true
}
// select thread
fn _Hg(_command : &str, _args : &Vec<&str>) -> bool
{
    encoder::reply_ok();
    true
}
// select thread
fn _Hc(_command : &str, _args : &Vec<&str>) -> bool
{
    encoder::reply_ok();   
    true
}

// Read registers
fn _g(_command : &str, _args : &Vec<&str>) -> bool
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
            crate::parsing_util::u8_to_ascii_to_buffer((reg &0xff) as u8 ,&mut buffer[2*j..]); 
            reg = reg >> 8;
        }
        e.add_u8(&buffer);
    }
    e.end();
    true
}
//
// Request reason for halt
fn _mark(_command : &str, _args : &Vec<&str>) -> bool
{
    //NOTARGET
    encoder::simple_send("W00");
    true
}


// EOF

