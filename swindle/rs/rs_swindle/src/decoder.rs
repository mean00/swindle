//! GDB packet decoder — state machine for parsing the GDB remote protocol.
//!
//! Implements a finite-state automaton that parses the GDB remote serial
//! protocol byte-by-byte. It handles:
//!
//! - GDB command packets (`$...` with checksum)
//! - RPC packets (`!...!`)
//! - Escape sequences (`}`)
//! - Checksum verification
//!
//! The automaton states are:
//! - `Idle`: waiting for `$` (GDB) or `!` (RPC) start character
//! - `Body`/`RpcBody`: accumulating packet bytes
//! - `End1`/`End2`: reading the 2-hex-digit checksum
//! - `Done`/`RpcDone`: complete packet ready for dispatch

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

use crate::packet_symbols::{
    CHAR_END, CHAR_ESCAPE, CHAR_RESET_04, CHAR_START, RPC_END, RPC_START, RPC_START_SESSION,
};
use crate::parsing_util::ascii_octet_to_hex;
use crate::rpc_wire::{ESCAPE_XOR, RPC_ESCAPE};
//
setup_log!(false);
//use crate::{bmplog, bmpwarning};
//
//
#[derive(PartialEq, Clone, Copy)]
enum PARSER_AUTOMATON {
    Idle,
    Init,
    Body,
    Escape,
    End1,
    End2,
    Done,
    RpcDone,
    Reset,
    RpcBody,
    Rpc2Head1, // alternat RPC starting by a '+'
    Error,
}
/// Result of a single `parse()` call.
///
/// - `Continue`: need more data
/// - `Ready`: a complete GDB packet is available
/// - `RpcReady`: a complete RPC packet is available
/// - `Reset`: a reset character was received
/// - `Error`: checksum mismatch or buffer overflow
#[derive(PartialEq)]
pub enum RESULT_AUTOMATON {
    Continue,
    Ready,
    RpcReady,
    Reset,
    Error,
}

/// GDB packet stream decoder.
///
/// A state machine that processes incoming bytes and extracts complete
/// GDB or RPC packets. Generic over `INPUT_BUFFER_SIZE` to allow
/// compile-time buffer sizing.
///
///
#[derive(PartialEq)]
pub enum GDB_STREAM_STATE {
    None,
    Ready,
    Cleaning,
}
pub struct gdb_stream<const INPUT_BUFFER_SIZE: usize> {
    state: GDB_STREAM_STATE,
    automaton: PARSER_AUTOMATON,
    input_buffer: [u8; INPUT_BUFFER_SIZE],
    indx: usize,
    checksum: usize,
    checksum_received: [u8; 2],
    /// An RPC body byte arrived escaped (`}` prefix): XOR the next one.
    escaped: bool,
}
impl<const INPUT_BUFFER_SIZE: usize> gdb_stream<INPUT_BUFFER_SIZE> {
    /// Create a new decoder in the `Idle` state.
    pub const fn new() -> Self {
        gdb_stream {
            state: GDB_STREAM_STATE::None,
            automaton: PARSER_AUTOMATON::Idle,
            input_buffer: [0; INPUT_BUFFER_SIZE],
            indx: 0,
            checksum: 0,               // only 1 byte is actually used
            checksum_received: [0, 0], // 2 Hex digits
            escaped: false,
        }
    }
    /// Mark the decoder as available or unavailable.
    pub fn set_ready(&mut self) {
        self.state = GDB_STREAM_STATE::Ready;
    }
    pub fn set_not_ready(&mut self) {
        self.state = GDB_STREAM_STATE::Cleaning;
    }
    pub fn check(&mut self) {
        if self.state == GDB_STREAM_STATE::Cleaning {}
    }
    /// Check if the decoder is available for use.
    pub fn get_available(&mut self) -> bool {
        self.state == GDB_STREAM_STATE::Ready
    }
    /// Reset the decoder to its initial idle state.
    pub fn init(&mut self) {
        self.state = GDB_STREAM_STATE::None;
        self.automaton = PARSER_AUTOMATON::Idle;
        self.input_buffer.fill(0);
        self.indx = 0;
        self.checksum = 0;
        self.checksum_received = [0, 0];
        self.escaped = false;
    }
    /// Reset the automaton to `Idle` without clearing the buffer.
    pub fn reset(&mut self) {
        self.automaton = PARSER_AUTOMATON::Idle;
    }
    /// Get the decoded packet and reset for the next one.
    ///
    /// Returns a slice of the internal buffer containing the complete packet
    /// body (without start/end markers or checksum).
    pub fn get_result(&mut self) -> &[u8] {
        self.automaton = PARSER_AUTOMATON::Idle;
        &self.input_buffer[0..self.indx]
    }

    /// Feed bytes into the decoder state machine.
    ///
    /// Processes `data` byte-by-byte through the automaton. Returns
    /// `(consumed, result)` where `consumed` is the number of bytes
    /// processed and `result` indicates the outcome.
    ///
    /// When `result` is `Ready` or `RpcReady`, call `get_result()` to
    /// retrieve the decoded packet.
    pub fn parse(&mut self, data: &[u8]) -> (usize, RESULT_AUTOMATON) {
        let mut dex = 0;
        let mut sz = data.len();
        let mut consumed = 0;

        bmplog!("In size {} ", data.len() as u32);

        // auto clear errors
        if self.automaton == PARSER_AUTOMATON::Error {
            self.automaton = PARSER_AUTOMATON::Idle;
        }

        while sz > 0 {
            let c: u8 = data[dex];
            bmplog!("InputChar, {}", c as char);
            //let _d : char = c as char;
            //bmplog!("c",d);
            //bmplog!("u",c);
            //bmplog!("S",self.automaton as usize);
            //bmplog!("i",self.indx);
            consumed += 1;
            sz -= 1;
            dex += 1;
            self.automaton = match self.automaton {
                PARSER_AUTOMATON::Init => PARSER_AUTOMATON::Init,
                //PARSER_AUTOMATON::Init => match c {
                //   CHAR_RESET_04   => PARSER_AUTOMATON::Reset,
                //  _ => PARSER_AUTOMATON::Init,
                //},
                PARSER_AUTOMATON::Idle => {
                    bmplog!("Idle\n");
                    match c
                                            {
                                               // RPC_START_SESSION /* + */ => PARSER_AUTOMATON::PARSER_AUTOMATON_RPC2_HEAD1,
                                                CHAR_START /*'$'*/      => {self.indx = 0;self.checksum=0;PARSER_AUTOMATON::Body},
                                                RPC_START  /* '!' */    => {
                                                                            bmplog!("rpc start\n");
                                                                            self.indx=0;
                                                                            self.escaped=false;
                                                                            PARSER_AUTOMATON::RpcBody
                                                                            },
                                                _                       => PARSER_AUTOMATON::Idle,
                                            }
                }
                PARSER_AUTOMATON::Rpc2Head1 => {
                    bmplog!("RPC_H1\n");
                    match c {
                        RPC_END => PARSER_AUTOMATON::Idle, /* Skip + EOM so we get a vanilla RPC string */
                        _ => PARSER_AUTOMATON::Idle, /* It is also used from some jtag RPC commands, dont optimize it */
                    }
                }
                PARSER_AUTOMATON::RpcBody => {
                    bmplog!("RPC_body\n");
                    match c
                                            {
                                                // `}` escapes the next body byte (XOR 0x20): the
                                                // CBOR payload is binary, so `#`, `!`, `$`, `{`
                                                // and `}` itself only ever arrive escaped.
                                                RPC_ESCAPE if !self.escaped => {
                                                                        self.escaped = true;
                                                                        PARSER_AUTOMATON::RpcBody
                                                                        },
                                                RPC_END /*'#'*/ if !self.escaped => {
                                                                        bmplog!("rpc done( {} )\n",self.indx);
                                                                        PARSER_AUTOMATON::RpcDone
                                                                        },
                                                RPC_START /*'!'*/ if !self.escaped => {self.indx=0;PARSER_AUTOMATON::RpcBody},  // restart ? wtf ?
                                                _                       => {
                                                                        let byte = if self.escaped {
                                                                            self.escaped = false;
                                                                            c ^ ESCAPE_XOR
                                                                        } else {
                                                                            c
                                                                        };
                                                                        if self.indx >= INPUT_BUFFER_SIZE
                                                                        {
                                                                            bmplog!("RPC input buffer overflow\n");
                                                                            PARSER_AUTOMATON::Error
                                                                        }else {
                                                                            self.input_buffer[self.indx]= byte;
                                                                            self.indx+=1;
                                                                            PARSER_AUTOMATON::RpcBody
                                                                        }
                                                                    },
                                            }
                }
                PARSER_AUTOMATON::Body => {
                    bmplog!("GDB_body\n");
                    match c
                                            {
                                                CHAR_START /*'$'*/  => {
                                                            bmplog!("RESTARTING DECODER\n");
                                                            self.indx = 0;
                                                            self.checksum=0;
                                                            PARSER_AUTOMATON::Body},
                                                CHAR_END /*'#'*/        => PARSER_AUTOMATON::End1,
                                                CHAR_ESCAPE /*'}'*/     => PARSER_AUTOMATON::Escape,
                                            //    CHAR_RESET_04           => PARSER_AUTOMATON::Reset,
                                                _                       => {
                                                                        bmplog!("T:<{}>",c);
                                                                        self.checksum+=c as usize;
                                                                        if self.indx >= INPUT_BUFFER_SIZE {
                                                                            bmplog!("GDB input buffer overflow\n");
                                                                            PARSER_AUTOMATON::Error
                                                                        } else {
                                                                            self.input_buffer[self.indx] = c;
//                                                                        self.input_buffer[self.indx]= match c
//                                                                        {
//                                                                            b'\t' => b' ',
//                                                                            _     => c,
//                                                                        };
                                                                            self.indx+=1;
                                                                            PARSER_AUTOMATON::Body
                                                                        }
                                                                    },
                                            }
                }
                PARSER_AUTOMATON::Escape => {
                    self.checksum += CHAR_ESCAPE as usize;
                    self.checksum += c as usize;
                    if self.indx >= INPUT_BUFFER_SIZE {
                        bmplog!("GDB input buffer overflow\n");
                        PARSER_AUTOMATON::Error
                    } else {
                        self.input_buffer[self.indx] = c ^ 0x20;
                        self.indx += 1;
                        PARSER_AUTOMATON::Body
                    }
                }
                PARSER_AUTOMATON::End1 => {
                    bmplog!("CHK1\n");
                    self.checksum_received[0] = c;
                    PARSER_AUTOMATON::End2
                }
                PARSER_AUTOMATON::End2 => {
                    bmplog!("CHK2\n");
                    self.checksum_received[1] = c;
                    // Verify checksum
                    let chk =
                        ascii_octet_to_hex(self.checksum_received[0], self.checksum_received[1]);
                    if chk == (self.checksum & 0xff) as u8 {
                        PARSER_AUTOMATON::Done
                    } else {
                        bmplog!("Wrong checksum\n");
                        PARSER_AUTOMATON::Error
                    }
                }
                PARSER_AUTOMATON::Reset => panic!("automatonReset"),
                PARSER_AUTOMATON::RpcDone => panic!("automatonRpcDone"), //; PARSER_AUTOMATON::Done},
                PARSER_AUTOMATON::Done => panic!("automatonDone"), //; PARSER_AUTOMATON::Done},
                PARSER_AUTOMATON::Error => panic!("automatonError"), //; PARSER_AUTOMATON::Done},
            };
            // should exit the loop even if we have data left ?
            match self.automaton {
                PARSER_AUTOMATON::Error => {
                    self.automaton = PARSER_AUTOMATON::Idle;
                    return (consumed, RESULT_AUTOMATON::Error);
                }
                PARSER_AUTOMATON::Done => {
                    return (consumed, RESULT_AUTOMATON::Ready);
                }
                PARSER_AUTOMATON::RpcDone => {
                    return (consumed, RESULT_AUTOMATON::RpcReady);
                }
                PARSER_AUTOMATON::Reset => {
                    bmplog!("RESET");
                    self.automaton = PARSER_AUTOMATON::Idle;
                    return (consumed, RESULT_AUTOMATON::Reset);
                }
                _ => (),
            }
        }
        (consumed, RESULT_AUTOMATON::Continue)
    }
}
//

#[cfg(test)]
mod tests {
    use super::*;
    use std::format;
    use std::vec::Vec;
    use crate::rpc_wire::escape_and_send;

    /// Run a whole frame through the automaton the way `run_parser()` does and
    /// return what a completed packet decodes to.
    fn payload(mut frame: &[u8]) -> (Vec<u8>, RESULT_AUTOMATON) {
        let mut stream = gdb_stream::<256>::new();
        let mut out = Vec::new();
        let mut outcome = RESULT_AUTOMATON::Continue;
        while !frame.is_empty() {
            let (consumed, result) = stream.parse(frame);
            assert!(consumed > 0, "the automaton must always make progress");
            if matches!(result, RESULT_AUTOMATON::Ready | RESULT_AUTOMATON::RpcReady) {
                out = stream.get_result().to_vec();
                outcome = result;
            }
            frame = &frame[consumed..];
        }
        (out, outcome)
    }

    #[test]
    fn rpc_body_unescapes_the_frame_markers() {
        // A CBOR payload is binary: every marker below has to come back as-is.
        let data = [0x23u8, 0x21, 0x7d, 0x24, 0x7b, 0x00, 0xff];
        let mut frame = Vec::from(*b"!GZ");
        escape_and_send(&data, &mut |chunk| frame.extend_from_slice(chunk));
        frame.push(RPC_END);
        frame.extend_from_slice(b"9A"); // the hex checksum, ignored by the RPC path

        let (body, outcome) = payload(&frame);
        assert!(matches!(outcome, RESULT_AUTOMATON::RpcReady));
        let mut expected = Vec::from(*b"GZ");
        expected.extend_from_slice(&data);
        assert_eq!(body, expected);
    }

    #[test]
    fn rpc_body_restarts_on_an_unescaped_start_marker() {
        // `!` inside the body (unescaped) restarts the packet, so only what
        // follows it survives — the host always escapes it.
        let mut frame = Vec::from(*b"!GZjunk!GZ");
        frame.push(RPC_END);
        let (body, outcome) = payload(&frame);
        assert!(matches!(outcome, RESULT_AUTOMATON::RpcReady));
        assert_eq!(body, Vec::from(*b"GZ"));
    }

    #[test]
    fn gdb_packets_are_unaffected() {
        let body = b"OK";
        let checksum: u8 = body.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        let frame = format!("${}#{checksum:02x}", "OK");
        let (packet, outcome) = payload(frame.as_bytes());
        assert!(matches!(outcome, RESULT_AUTOMATON::Ready));
        assert_eq!(packet, body.to_vec());
    }
}

// EOF
