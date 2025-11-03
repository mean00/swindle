// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec::Vec;

use super::{CommandTree, exec_one};
use crate::encoder::encoder;
use crate::packet_symbols::INPUT_BUFFER_SIZE;

use crate::bmp;
use crate::bmp::MemoryBlock;
use crate::bmp::bmp_get_mapping;
use crate::bmp::mapping::{Flash, Ram};
use crate::commands::CallbackType;
use crate::commands::mon::_qRcmd;
use crate::commands::q_thread::{
    _qC, _qP, _qSymbol, _qThreadExtraInfo, _qfThreadInfo, _qsThreadInfo,
};
use crate::crc::abstract_crc32;
use crate::parsing_util::ascii_string_hex_to_u32;

use numtoa::NumToA;

crate::setup_log!(false);
use crate::bmplog;

const q_command_tree: [CommandTree; 18] = [
    // lldb specific
    CommandTree {
        command: "qLaunchGDBServer",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_qLaunchGDBServer),
        start_separator: ":",
        next_separator: ":",
    },
    CommandTree {
        command: "qHostInfo",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_qHostInfo),
        start_separator: "",
        next_separator: "",
    },
    CommandTree {
        command: "qQueryGDBServer",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_qQueryGDBServer),
        start_separator: "",
        next_separator: "",
    },
    CommandTree {
        command: "qGetWorkingDir",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_qGetWorkingDir),
        start_separator: "",
        next_separator: "",
    },
    CommandTree {
        command: "qRegisterInfo",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_qRegisterInfo),
        start_separator: "#",
        next_separator: "",
    },
    //
    CommandTree {
        command: "qSymbol",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_qSymbol),
        start_separator: ":",
        next_separator: ":",
    }, // read symbol
    CommandTree {
        command: "qSupported", // : qSupported:multiprocess+;swbreak+;hwbreak+;qRelocInsn+;fork-ev
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_qSupported),
        start_separator: ":",
        next_separator: ";",
    }, // supported features
    CommandTree {
        command: "qXfer", // qXfer:features:read:target.xml:d9c,83b
        min_args: 3,
        require_connected: false,
        cb: CallbackType::text(_qXfer),
        start_separator: ":",
        next_separator: ":",
    }, // read memory map
    CommandTree {
        command: "qTStatus",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_qTStatus),
        start_separator: "",
        next_separator: "",
    }, // trace status
    CommandTree {
        command: "qRcmd",
        min_args: 1,
        require_connected: false,
        cb: CallbackType::text(_qRcmd),
        start_separator: ",",
        next_separator: "",
    }, // execute command
    CommandTree {
        command: "qAttached",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_qAttached),
        start_separator: "",
        next_separator: "",
    }, // remote thread
    CommandTree {
        command: "qfThreadInfo",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_qfThreadInfo),
        start_separator: "",
        next_separator: "",
    }, // thread info begin
    CommandTree {
        command: "qsThreadInfo",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_qsThreadInfo),
        start_separator: "",
        next_separator: "",
    }, // List threads
    CommandTree {
        command: "qThreadExtraInfo",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_qThreadExtraInfo),
        start_separator: "",
        next_separator: "",
    }, // List threads
    CommandTree {
        command: "qP",
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_qP),
        start_separator: "",
        next_separator: "",
    }, // List threads
    CommandTree {
        command: "qCRC", //  qCRC:0,61b4
        min_args: 0,
        require_connected: true,
        cb: CallbackType::text(_qCRC),
        start_separator: ":",
        next_separator: ",",
    }, // set code/data/.. offset
    CommandTree {
        command: "qC",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_qC),
        start_separator: "",
        next_separator: "",
    }, // Current thread Id
    CommandTree {
        command: "qOffsets",
        min_args: 0,
        require_connected: false,
        cb: CallbackType::text(_qOffsets),
        start_separator: "",
        next_separator: "",
    }, // set code/data/.. offset
];

//
//
//

#[unsafe(no_mangle)]
pub fn _q(command: &str, args: &[u8]) -> bool {
    exec_one(&q_command_tree, command, args)
}
//
//  qSupported:multiprocess+;swbreak+;hwbreak+;qRelocInsn+;fork-ev
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
// qXfer:features:read:target.xml:d9c,83b

fn _qXfer(_command: &str, args: &[&str]) -> bool {
    match (args[0], args[1], args[2]) {
        ("memory-map", "read", "") => _qXfer_memory_map(args[3]),
        ("features", "read", "target.xml") => _qXfer_features_regs(args[3]),
        _ => false,
    }
}

//
//
fn validate_q_query(arg: &str) -> Option<(usize, usize)> {
    let conf: Vec<&str> = arg.split(',').collect();
    if conf.len() != 2 {
        return None;
    }
    //
    let start_address: usize = ascii_string_hex_to_u32(conf[0]) as usize;
    let length: usize = ascii_string_hex_to_u32(conf[1]) as usize;
    Some((start_address, length))
}
//
//  read target.xml [offset,size]
//
fn _qXfer_features_regs(arg: &str) -> bool {
    let start_address: usize;
    let length: usize;
    match validate_q_query(arg) {
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
    let reply = bmp::bmp_register_description();
    let reply_size = reply.len();

    if reply_size == 0 {
        bmp::bmp_drop_register_description();
        encoder::reply_e01();
        return true;
    }

    if start_address >= reply_size {
        bmp::bmp_drop_register_description();
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
    bmp::bmp_drop_register_description();
    true
}
//
//  memory-map:read::0,50d
//
fn _qXfer_memory_map(arg: &str) -> bool {
    let start_address: usize;
    let _length: usize;
    match validate_q_query(arg) {
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
/*
 *  ARM64  : 16777228    0x100000c
 *  ARM    : 12          0x0c
 *          CPU_SUBTYPE_ARM_V6M     14  0xe
 *          CPU_SUBTYPE_ARM_V7M     15  0xf
 *          CPU_SUBTYPE_ARM_V7EM    16  0x10
 *          CPU_SUBTYPE_ARM_V8M     17  0x11
 *  RV32   : 24          0x18
 *          CPU_SUBTYPE_RISCV_IM    3  0x03
 *  RV64   : 16777240    0x1000018
 *
 *
 */
#[unsafe(no_mangle)]
fn _qHostInfo(_command: &str, _args: &[&str]) -> bool {
    let mut e = encoder::new();
    e.begin();
    e.add("cputype:12;");
    e.add("cpusubtype:15;");
    e.add("ostype:none;");
    e.add("vendor:unknown;");
    e.add("endian:little;ptrsize:4;");
    e.end();
    true
}
fn _qLaunchGDBServer(_command: &str, _args: &[&str]) -> bool {
    encoder::simple_send("pid:1;port:2001");
    true
}
fn _qGetWorkingDir(_command: &str, _args: &[&str]) -> bool {
    encoder::simple_send("");
    true
}
fn _qRegisterInfo(_command: &str, _args: &[&str]) -> bool {
    encoder::simple_send(
        "name:r0;bitsize:32;offset:0;encoding:uint;format:hex;set:General Purpose;generic:arg1;",
    );
    true
}
fn _qQueryGDBServer(_command: &str, _args: &[&str]) -> bool {
    encoder::simple_send("");
    true
}

// get offset
fn _qOffsets(_command: &str, _args: &[&str]) -> bool {
    encoder::simple_send("Text=0;Data=0;Bss=0");
    true
}
/**
 * compute crc32 over bit of memory
 *  qCRC:0,61b4
 */
fn _qCRC(_command: &str, args: &[&str]) -> bool {
    if args.len() != 2 {
        encoder::reply_e01();
        return true;
    }

    let address = ascii_string_hex_to_u32(args[0]);
    let length = ascii_string_hex_to_u32(args[1]);

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
