// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec;
use alloc::vec::Vec;

use super::{exec_one, CommandTree};
use crate::encoder::encoder;
use crate::packet_symbols::INPUT_BUFFER_SIZE;

use super::mon::_swdp_scan;
use crate::bmp::bmp_attached;
use crate::bmp::bmp_crc32;
use crate::bmp::bmp_get_mapping;
use crate::bmp::mapping::{Flash, Ram};
use crate::bmp::MemoryBlock;
use crate::commands::mon::_qRcmd;
use crate::commands::q_thread::{
    _qC, _qP, _qSymbol, _qThreadExtraInfo, _qfThreadInfo, _qsThreadInfo,
};
use crate::commands::CallbackType;
use crate::crc::abstract_crc32;
use crate::parsing_util;
use crate::util::xmin;

use numtoa::NumToA;

crate::setup_log!(false);
use crate::{bmplog, bmpwarning};

const q_command_tree: [CommandTree; 13] = [
    CommandTree {
        command: "qSymbol",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_qSymbol),
    }, // read symbol
    CommandTree {
        command: "qSupported",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_qSupported),
    }, // supported features
    CommandTree {
        command: "qXfer",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_qXfer),
    }, // read memory map
    CommandTree {
        command: "qTStatus",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_qTStatus),
    }, // trace status
    CommandTree {
        command: "qRcmd",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_qRcmd),
    }, // execute command
    CommandTree {
        command: "qAttached",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_qAttached),
    }, // remote thread
    CommandTree {
        command: "qfThreadInfo",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_qfThreadInfo),
    }, // thread info begin
    CommandTree {
        command: "qsThreadInfo",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_qsThreadInfo),
    }, // List threads
    CommandTree {
        command: "qThreadExtraInfo",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_qThreadExtraInfo),
    }, // List threads
    CommandTree {
        command: "qP",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_qP),
    }, // List threads
    CommandTree {
        command: "qCRC",
        args: 0,
        require_connected: true,
        cb: CallbackType::text(_qCRC),
    }, // set code/data/.. offset
    CommandTree {
        command: "qC",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_qC),
    }, // Current thread Id
    CommandTree {
        command: "qOffsets",
        args: 0,
        require_connected: false,
        cb: CallbackType::text(_qOffsets),
    }, // set code/data/.. offset
];

//
//
//

pub fn _q(command: &str, args: &[u8]) -> bool {
    exec_one(&q_command_tree, command, args)
}
//
//
//
fn _qSupported(_command: &str, _args: &[&str]) -> bool {
    let mut buffer: [u8; 20] = [0; 20]; // should be big enough!

    let mut e = encoder::new();
    e.begin();
    e.add("PacketSize=");
    e.add(INPUT_BUFFER_SIZE.numtoa_str(16, &mut buffer)); // INPUT_BUFFER_SIZE
    e.add(";hwbreak+;swbreak-");
    e.add(";qXfer:memory-map:read+;qXfer:features:read+;vCont+");
    e.end();
    true
}
//
// Read memory map
//
// the full command is qXfer:features:read:target.xml:0,50d
// we assume it fits (?)
const height: &str = "00000000";
fn hex8(digit: u32, buffer: &mut [u8], e: &mut encoder) {
    let pfix: &str = digit.numtoa_str(16, buffer);
    if pfix.len() < 8 {
        e.add(&height[0..(8 - pfix.len())]);
    }
    e.add(pfix);
}
//
//
fn _qXfer(_command: &str, args: &[&str]) -> bool {
    if args.len() < 3 {
        encoder::reply_e01();
        return true;
    }

    match args[0] {
        "memory-map" => _qXfer_memory_map(&args[1..]),
        "features" => _qXfer_features_regs(&args[1..]),
        _ => false,
    }
}

//
//
fn validate_q_query(args: &[&str], header1: &str, header2: &str) -> Option<(usize, usize)> {
    if args.len() != 3 {
        return None;
    }
    if args[0] != header1 || args[1] != header2 {
        return None;
    }
    let conf: Vec<&str> = args[2].split(',').collect();
    if conf.len() != 2 {
        return None;
    }
    //
    let start_address: usize = crate::parsing_util::ascii_string_to_u32(conf[0]) as usize;
    let length: usize = crate::parsing_util::ascii_string_to_u32(conf[1]) as usize;
    Some((start_address, length))
}
//
//  read target.xml [offset,size]
//
fn _qXfer_features_regs(args: &[&str]) -> bool {
    let start_address: usize;
    let length: usize;
    match validate_q_query(args, "read", "target.xml") {
        None => {
            encoder::reply_e01();
            return true;
        }
        Some((a, b)) => {
            start_address = a;
            length = b;
        }
    }

    // offset & size in hex
    let reply = crate::bmp::bmp_register_description();
    let reply_size = reply.len();

    if reply_size == 0 {
        crate::bmp::bmp_drop_register_description();
        encoder::reply_e01();
        return true;
    }

    if start_address >= reply_size {
        crate::bmp::bmp_drop_register_description();
        encoder::simple_send("l");
        return true;
    }

    //
    let mut e = encoder::new();
    e.begin();
    e.add("m");
    let end_pos = core::cmp::min(start_address + length, reply_size);
    e.add(&reply[start_address..end_pos]);
    e.end();
    crate::bmp::bmp_drop_register_description();
    true
}
//
//  memory-map:read::0,50d
//
fn _qXfer_memory_map(args: &[&str]) -> bool {
    let start_address: usize;
    let _length: usize;
    match validate_q_query(args, "read", "") {
        None => return false,
        Some((a, b)) => {
            start_address = a;
            _length = b;
        }
    }
    if start_address != 0 {
        encoder::simple_send("l");
        return true;
    }

    let mut e = encoder::new();
    let mut buffer: [u8; 20] = [0; 20];

    e.begin();
    e.add("m<memory-map>");

    {
        let ram: Vec<MemoryBlock> = bmp_get_mapping(Ram);
        for i in ram {
            e.add("<memory type=\"ram\" start=\"0x");
            hex8(i.start_address, &mut buffer, &mut e);
            e.add("\" length=\"0x");
            hex8(i.length, &mut buffer, &mut e);
            e.add("\"/>");
        }
    }
    {
        let flash: Vec<MemoryBlock> = bmp_get_mapping(Flash);
        for i in flash {
            e.add("<memory type=\"flash\" start=\"0x");
            hex8(i.start_address, &mut buffer, &mut e);
            e.add("\" length=\"0x");
            hex8(i.length, &mut buffer, &mut e);
            e.add("\">");
            e.add("<property name=\"blocksize\">0x");
            hex8(i.block_size, &mut buffer, &mut e);
            e.add("</property></memory>");
        }
    }
    e.add("</memory-map>");
    e.end();
    true
}
//
// Trace
//
fn _qTStatus(_command: &str, _args: &[&str]) -> bool {
    false
}

//
// Execute command
//
fn _qAttached(_command: &str, _args: &[&str]) -> bool {
    encoder::simple_send("1");
    true
}

// get offset
fn _qOffsets(_command: &str, _args: &[&str]) -> bool {
    encoder::simple_send("Text=0;Data=0;Bss=0");
    true
}
/**
 * compute crc32 over bit of memory
 */
fn _qCRC(_command: &str, args: &[&str]) -> bool {
    if args.is_empty() {
        encoder::reply_e01();
        return true;
    }

    let address: u32;
    let length: u32;

    match crate::parsing_util::take_adress_length(args[0]) {
        Some((x, y)) => {
            address = x;
            length = y;
        }
        None => {
            encoder::reply_e01();
            return true;
        }
    }

    let mut crc: u32 = 0;
    let status = abstract_crc32(address, length, &mut crc); // remote

    if !status {
        encoder::reply_e01();
        return false;
    }

    bmplog!("crc::<0x{:x}>\n", crc);
    {
        let mut e = encoder::new();
        e.begin();
        e.add("C");
        let mut buffer: [u8; 10] = [0; 10];
        let hex = crc.numtoa_str(16, &mut buffer);
        e.add(hex); // INPUT_BUFFER_SIZE
        e.end();
    }
    true
}

// EOF
