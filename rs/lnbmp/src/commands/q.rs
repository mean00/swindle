
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::util::hex_to_u8s;

use crate::encoder::encoder;
use super::{CommandTree,exec_one};
use crate::packet_symbols::INPUT_BUFFER_SIZE;

use super::mon::{_swdp_scan};
use crate::bmp::bmp_attached;
use crate::bmp::bmp_get_mapping;
use crate::bmp::MemoryBlock;
use crate::bmp::mapping::{ FLASH,RAM};


use numtoa::NumToA;

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
    let mut buffer: [u8;20] = [0; 20]; // should be big enough!    

    let mut e = encoder::new();
    e.begin();
    e.add("PacketSize=");    
    e.add(INPUT_BUFFER_SIZE.numtoa_str(10,&mut buffer));     // INPUT_BUFFER_SIZE
    e.add(";qXfer:memory-map:read+;qXfer:features:read+");
    e.end();
    return true;
}
//
// Read memory map
//
// the full command is qXfer:features:read:target.xml:0,50d
// we assume it fits (?)
const height : &str = "00000000";
fn hex8(digit : u32, buffer : &mut [u8], e: &mut encoder)
{
    let pfix : &str = digit.numtoa_str(16,buffer);
    if pfix.len() < 8
    {
        e.add( &height[0..(8-pfix.len())]);
    }
    e.add(pfix);
}
//
//
fn _qXfer(_tokns : &Vec<&str>) -> bool
{
    

    if !crate::bmp::bmp_attached()
    {
        encoder::simple_send("E01");    
        return true;
    }
    
    if _tokns.len() ==0
    {
        return false;
    }

    let args : Vec <&str>= _tokns[0].split(':').collect();
    if args.len()< 2
    {
        return false;
    }
    match args[1]
    {
        "memory-map" => return _qXfer_memory_map(_tokns),
        "features"   => return _qXfer_features_regs(_tokns),
        _            => return false,
    }    
}
//
//
fn _qXfer_features_regs(_tokns : &Vec<&str>) -> bool
{
    let mut e = encoder::new();
    e.begin();
    e.add("m");
    e.add(crate::bmp::bmp_register_description());
    e.end();
    true
}
// 
fn _qXfer_memory_map(_tokns : &Vec<&str>) -> bool
{
    if !crate::bmp::bmp_attached()
    {
        encoder::simple_send("E01");    
        return true;
    }
    
    if _tokns.len() ==0
    {
        return false;
    }

    // this is a weak hack to detect if it is the 2nd query
    // i.e. the offset is not zero
    // we assume we replied all in the 1st reply
    if !(_tokns[0].starts_with("qXfer:features:read:target.xml:0"))
    {
        encoder::simple_send("l");   
        return true;
    }


    let mut e = encoder::new();
    let mut buffer : [u8;20] = [0;20];
    
    e.begin();
    e.add("m<memory-map>");

    {
        let ram : Vec<MemoryBlock> = bmp_get_mapping(RAM);
        for i in 0..ram.len()
        {
            e.add("<memory type=\"ram\" start=\"0x");
            hex8(ram[i].start_address, &mut buffer, &mut e);
            e.add("\" length=\"0x");
            hex8(ram[i].length, &mut buffer, &mut e);            
            e.add("\"/>");
        }
    }
    {
        let flash : Vec<MemoryBlock> = bmp_get_mapping(FLASH );
        for i in 0..flash.len()
        {
            e.add("<memory type=\"flash\" start=\"0x");
            hex8(flash[i].start_address, &mut buffer, &mut e);
            e.add("\" length=\"0x");            
            hex8(flash[i].length, &mut buffer, &mut e);
            e.add("\">");
            e.add("<property name=\"blocksize\">0x");
            hex8(flash[i].block_size, &mut buffer, &mut e);            
            e.add("</property></memory>");
        }
    }
    e.add("</memory-map>");
    e.end();
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