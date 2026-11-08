//! Memory map XML generator (`qXfer:memory-map:read`)
//!
//! Generates GDB XML memory map descriptions directly in Rust without heap allocation.
//!
//! Format produced:
//! ```xml
//! <memory-map>
//!   <memory type="ram" start="0x20000000" length="0x00010000"/>
//!   <memory type="flash" start="0x08000000" length="0x00080000">
//!     <property name="blocksize">0x00000800</property>
//!   </memory>
//! </memory-map>
//! ```

use crate::bmp;
use crate::encoder::encoder;
use crate::format_util::{hex_digits, HexCase, MAX_HEX_DIGITS};
use crate::target_xml::XmlSink;

const MAP_OPEN: &str = "<memory-map>";
const MAP_CLOSE: &str = "</memory-map>";

const RAM_START: &str = "<memory type=\"ram\" start=\"0x";
const FLASH_START: &str = "<memory type=\"flash\" start=\"0x";
const MID_LENGTH: &str = "\" length=\"0x";
const RAM_END: &str = "\"/>";
const FLASH_MID_BLOCKSIZE: &str = "\"><property name=\"blocksize\">0x";
const FLASH_END: &str = "</property></memory>";

/// Generates all memory map XML chunks via XmlSink.
fn for_each_memory_map_chunk(sink: &mut XmlSink) {
    if !bmp::bmp_attached() {
        return;
    }

    sink.write(MAP_OPEN);

    let mut hex_buf = [0u8; MAX_HEX_DIGITS];

    // 1. RAM entries
    let ram_count = bmp::bmp_map_count(bmp::mapping::Ram);
    for i in 0..ram_count {
        if let Some(block) = bmp::bmp_map_get(bmp::mapping::Ram, i) {
            // <memory type="ram" start="0x{start}" length="0x{size}"/>
            sink.write(RAM_START);
            sink.write(hex_digits(block.start_address, MAX_HEX_DIGITS, HexCase::Lower, &mut hex_buf));
            sink.write(MID_LENGTH);
            sink.write(hex_digits(block.length, MAX_HEX_DIGITS, HexCase::Lower, &mut hex_buf));
            sink.write(RAM_END);
        }
    }

    // 2. Flash entries
    let flash_count = bmp::bmp_map_count(bmp::mapping::Flash);
    for i in 0..flash_count {
        if let Some(block) = bmp::bmp_map_get(bmp::mapping::Flash, i) {
            // <memory type="flash" start="0x{start}" length="0x{size}"><property name="blocksize">0x{block_size}</property></memory>
            sink.write(FLASH_START);
            sink.write(hex_digits(block.start_address, MAX_HEX_DIGITS, HexCase::Lower, &mut hex_buf));
            sink.write(MID_LENGTH);
            sink.write(hex_digits(block.length, MAX_HEX_DIGITS, HexCase::Lower, &mut hex_buf));
            sink.write(FLASH_MID_BLOCKSIZE);
            sink.write(hex_digits(block.block_size, MAX_HEX_DIGITS, HexCase::Lower, &mut hex_buf));
            sink.write(FLASH_END);
        }
    }

    sink.write(MAP_CLOSE);
}

/// Compute total length of the memory map XML description.
pub fn get_memory_map_xml_len() -> usize {
    let mut sink = XmlSink::Count(0);
    for_each_memory_map_chunk(&mut sink);
    match sink {
        XmlSink::Count(n) => n,
        _ => 0,
    }
}

/// Send the XML range `[start_address..start_address + length]` through the GDB packet encoder.
pub fn send_memory_map_xml_chunk(e: &mut encoder, start_address: usize, length: usize) {
    let mut sink = XmlSink::Stream {
        start: start_address,
        end: start_address + length,
        offset: 0,
        encoder: e,
    };
    for_each_memory_map_chunk(&mut sink);
}

/// Dump the whole memory map XML (`qXfer:memory-map:read`, i.e. the RAM and flash blocks
/// GDB is told about) to the GDB console, one tag per line. Backs `mon xml memory`.
pub fn dump_memory_map_xml() {
    for_each_memory_map_chunk(&mut XmlSink::Console);
}
