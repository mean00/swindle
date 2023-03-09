
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
use crate::bmp::bmp_crc32;
use crate::bmp::mapping::{ FLASH,RAM};
use crate::commands::CallbackType;
use crate::commands::mon::_qRcmd;

use numtoa::NumToA;

const q_command_tree: [CommandTree;10] = 
[
    CommandTree{ command: "qSupported",args: 0,     require_connected: false, cb: CallbackType::text( _qSupported )},      // supported features
    CommandTree{ command: "qXfer",args: 0,          require_connected: false, cb: CallbackType::text( _qXfer) },           // read memory map
    CommandTree{ command: "qTStatus",args: 0,       require_connected: false, cb: CallbackType::text( _qTStatus )},        // trace status
    CommandTree{ command: "qRcmd",args: 0,          require_connected: false, cb: CallbackType::text( _qRcmd) },           // execute command
    CommandTree{ command: "qAttached",args: 0,      require_connected: false, cb: CallbackType::text( _qAttached )},       // remote thread
    CommandTree{ command: "qfThreadInfo",args: 0,   require_connected: false, cb: CallbackType::text( _qfThreadInfo) },   // thread info begin
    CommandTree{ command: "qsThreadInfo",args: 0,   require_connected: false, cb: CallbackType::text( _qsThreadInfo )},   // List threads
    CommandTree{ command: "qCRC",args: 0,           require_connected: true,  cb: CallbackType::text( _qCRC) },        // set code/data/.. offset        
    CommandTree{ command: "qC",args: 0,             require_connected: false, cb: CallbackType::text( _qC) },             // Current thread Id
    CommandTree{ command: "qOffsets",args: 0,       require_connected: false, cb: CallbackType::text( _qOffsets) },        // set code/data/.. offset        
];


//
//
//

pub fn _q(command : &str, args : &[u8]) -> bool
{
    return exec_one(&q_command_tree,command, args);
}
//
//
//
fn _qSupported(_command : &str, _args : &Vec<&str>) -> bool
{
    let mut buffer: [u8;20] = [0; 20]; // should be big enough!    

    let mut e = encoder::new();
    e.begin();
    e.add("PacketSize=");    
    e.add(INPUT_BUFFER_SIZE.numtoa_str(16,&mut buffer));     // INPUT_BUFFER_SIZE
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
fn _qXfer(_command : &str, args : &Vec<&str>) -> bool
{
    
    if args.len() < 3
    {
        encoder::reply_e01(); 
        return true;
    }

    match args[0]
    {
        "memory-map" => return _qXfer_memory_map(&args[1..]),
        "features"   => return _qXfer_features_regs(&args[1..]),
        _            => return false,
    }    
}

//
//
fn validate_q_query(args : &[&str], header1 : &str, header2 : &str ) -> Option<(usize,usize)>
{
    if args.len()!=3
    {
        return None;
    }
    if args[0]!= header1 || args[1]!= header2
    {
        return None;
    }
    let conf : Vec <&str>= args[2].split(',').collect();
    if conf.len()!=2
    {
        return None;
    }
    //
    let start_address : usize = crate::util::ascii_to_u32(conf[0]) as usize;
    let length : usize = crate::util::ascii_to_u32(conf[1]) as usize;
    return Some((start_address, length));
}
//
//  read target.xml [offset,size]
// 
fn _qXfer_features_regs(args : &[&str]) -> bool
{
    let start_address : usize ;
    let length : usize ;
    match validate_q_query(args,"read","target.xml")
    {
        None        => {encoder::reply_e01();return true;},
        Some((a,b)) => {start_address=a;length = b;},
    }

    // offset & size in hex
    let reply = crate::bmp::bmp_register_description();
    let reply_size = reply.len();

    if reply_size==0
    {
        encoder::reply_e01();
        return true;
    }

    if start_address >= reply_size
    {
        encoder::simple_send("l");
        return true;
    }
    
    //
    let mut e = encoder::new();
    e.begin();
    e.add("m");
    let end_pos = core::cmp::min(start_address+length,reply_size);
    e.add(&reply[start_address..end_pos]);
    e.end();
    true
}
// 
//  memory-map:read::0,50d
//
fn _qXfer_memory_map(args : &[&str]) -> bool
{
   
    let start_address : usize ;
    let _length : usize ;
    match validate_q_query(args,"read","")
    {
        None        => return false,
        Some((a,b)) => {start_address=a;_length = b;},
    }     
    if start_address !=0
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
fn _qTStatus(_command : &str, _args : &Vec<&str>) -> bool
{    
    return false;
}

//
// Execute command
//
fn _qAttached(_command : &str, _args : &Vec<&str>) -> bool
{    
    encoder::simple_send("1");    
    return true;
}
//
//
fn _qfThreadInfo(_command : &str, _args : &Vec<&str>) -> bool
{    
    encoder::simple_send("m1");    
    return true;
}
//
//
fn _qsThreadInfo(_command : &str, _args : &Vec<&str>) -> bool
{    
    encoder::simple_send("1");    
    return true;
}
// get current thread I
fn _qC(_command : &str, _args : &Vec<&str>) -> bool
{    
    encoder::simple_send("QC1");    
    return true;
}
// get offset
fn _qOffsets(_command : &str, _args : &Vec<&str>) -> bool
{    
    encoder::simple_send("Text=0;Data=0;Bss=0");    
    return true;
}
// qCRC:addr hex,length hex’
// return crc32 hex

fn _qCRC(_command : &str, args : &Vec<&str>) -> bool
{

    if args.len()==0
    {
        encoder::reply_e01();
        return true;
    }

    let mut address : u32 =0;
    let mut length : u32 = 0;

    match crate::util::take_adress_length(args[0])
    {
        Some( (x,y)) => { address = x;length = y;},
        None =>   {encoder::reply_e01(); return true;},
    }
    let mut buffer: [u8;20] = [0; 20]; // should be big enough!    
    let crc = bmp_crc32(address,length);
    match crc 
    {
        Some(x) => 
                    {
                        let mut e = encoder::new();
                        e.begin();
                        e.add("C");
                        e.add(x.numtoa_str(16,&mut buffer));     // INPUT_BUFFER_SIZE
                        e.end();
                    },
        None => encoder::error(3),
    }
    true

}




// EOF