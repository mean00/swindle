//! The wire half of the probe ↔ host RPC: framing, escaping, CBOR payload.
//!
//! Both ends of the RPC are generated from `rpc_protocol.toml` and both write
//! their payload with CBOR (`minicbor`). This module is the single description
//! of *how* a payload is framed and escaped, so the two ends cannot drift.
//!
//! ## Frame
//!
//! ```text
//! request  host  -> probe : '!' <escaped [class][cmd][cbor]> '#' <2 hex checksum>
//! reply    probe -> host  : '&' <status> <escaped [cbor]> '#'
//! ```
//!
//! `[class][cmd]` is still two raw bytes (see `rpc_protocol.toml`); the
//! parameters after it are one CBOR array with one element per `params` entry,
//! and a reply is one CBOR array with one element per reply field (an `ok`
//! reply carries no payload at all).
//!
//! ## Escaping
//!
//! A frame body may not contain any byte of [`must_escape`]: each of those is
//! sent as [`RPC_ESCAPE`] followed by the byte XOR [`ESCAPE_XOR`] — the GDB
//! remote protocol convention. The reply needs it because the host reader stops
//! at the first `#`; the request needs it because the probe's packet automaton
//! (`decoder.rs`) does exactly the same.

use crate::SyncCell;
use crate::packet_symbols::{CHAR_END, CHAR_ESCAPE, CHAR_ESCAPE2, CHAR_START, RPC_START};

/// Frame terminator (`#`).
pub const RPC_EOM: u8 = CHAR_END;
/// Escape prefix (`}`): the next byte is that byte XOR [`ESCAPE_XOR`].
pub const RPC_ESCAPE: u8 = CHAR_ESCAPE;
/// Mask applied to an escaped byte.
pub const ESCAPE_XOR: u8 = 0x20;

/// Bytes that cannot appear literally in a frame body: the two frame markers,
/// the GDB markers (a frame body is read by the same automaton) and the escape
/// prefix itself.
#[inline]
pub const fn must_escape(byte: u8) -> bool {
    byte == RPC_EOM
        || byte == RPC_START
        || byte == CHAR_START
        || byte == RPC_ESCAPE
        || byte == CHAR_ESCAPE2
}

/// Un-escape a frame body in place, returning the decoded length.
///
/// Only ever shrinks, so the caller may decode into the slice it read.
pub fn unescape_in_place(data: &mut [u8]) -> usize {
    let mut out = 0;
    let mut index = 0;
    while index < data.len() {
        let byte = data[index];
        if byte == RPC_ESCAPE && index + 1 < data.len() {
            data[out] = data[index + 1] ^ ESCAPE_XOR;
            index += 2;
        } else {
            data[out] = byte;
            index += 1;
        }
        out += 1;
    }
    out
}

/// Escaped output chunk size: an escape pair never straddles two sends.
const SCRATCH_SIZE: usize = 64;
// DF — not thread safe, not re-entrant, ugly but simple.
static SCRATCH: SyncCell<[u8; SCRATCH_SIZE]> = SyncCell::new([0; SCRATCH_SIZE]);

/// Escape `data` and hand it to `send` in pieces of at most [`SCRATCH_SIZE`] bytes.
pub fn escape_and_send(data: &[u8], send: &mut impl FnMut(&[u8])) {
    // SAFETY: single-shot use inside one frame (see the static above).
    let scratch = unsafe { SCRATCH.as_mut() };
    let mut used = 0;
    for &byte in data {
        if must_escape(byte) {
            scratch[used] = RPC_ESCAPE;
            scratch[used + 1] = byte ^ ESCAPE_XOR;
            used += 2;
        } else {
            scratch[used] = byte;
            used += 1;
        }
        if used > SCRATCH_SIZE - 2 {
            send(&scratch[..used]);
            used = 0;
        }
    }
    if used > 0 {
        send(&scratch[..used]);
    }
}

/// A [`minicbor`] writer that escapes its output into a frame body.
pub struct EscapingWriter<F: FnMut(&[u8])>(pub F);

impl<F: FnMut(&[u8])> minicbor::encode::Write for EscapingWriter<F> {
    type Error = core::convert::Infallible;

    fn write_all(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        escape_and_send(buf, &mut self.0);
        Ok(())
    }
}

/// Reads the CBOR parameters of one RPC request.
///
/// `next_cmd()` consumes the raw `[class][cmd]` header bytes, the accessors
/// then read the CBOR parameter array element by element. Malformed input never
/// panics: a missing or mistyped element reads as zero / empty / false, which
/// the command implementations treat as "nothing to do".
pub struct RpcParams<'a> {
    data: &'a [u8],
    pos: usize,
    decoder: minicbor::Decoder<'a>,
    in_array: bool,
}

impl<'a> RpcParams<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        RpcParams {
            data,
            pos: 0,
            decoder: minicbor::Decoder::new(data),
            in_array: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Packet bytes still unconsumed (header and CBOR together).
    pub fn len(&self) -> usize {
        self.data.len().saturating_sub(self.consumed())
    }

    fn consumed(&self) -> usize {
        if self.in_array {
            self.decoder.position().max(self.pos.min(self.data.len()))
        } else {
            self.pos
        }
    }

    /// Consume one raw header byte (the class byte, then the command byte).
    pub fn next_cmd(&mut self) -> u8 {
        let byte = self.data.get(self.pos).copied().unwrap_or(0);
        self.pos += 1;
        if self.pos <= self.data.len() {
            self.decoder.set_position(self.pos);
        }
        byte
    }

    /// The decoder, positioned inside the parameter array.
    fn params(&mut self) -> &mut minicbor::Decoder<'a> {
        if !self.in_array {
            self.in_array = true;
            let _ = self.decoder.array();
        }
        &mut self.decoder
    }

    pub fn next_u32(&mut self) -> u32 {
        self.params().u32().unwrap_or(0)
    }

    pub fn next_u8(&mut self) -> u32 {
        self.params().u8().unwrap_or(0) as u32
    }

    pub fn next_u16(&mut self) -> u32 {
        self.params().u16().unwrap_or(0) as u32
    }

    pub fn next_bool(&mut self) -> bool {
        self.params().bool().unwrap_or(false)
    }

    /// The remaining parameter as a byte string (`hex_block` parameters).
    pub fn bytes(&mut self) -> &'a [u8] {
        self.params().bytes().unwrap_or(&[])
    }

    /// The remaining parameter as a byte string.
    ///
    /// `hex_block` used to be "the rest of the packet", hence the historical
    /// name the generated dispatch code uses.
    pub fn end(&mut self) -> &'a [u8] {
        self.bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minicbor::Encoder;
    use minicbor::encode::write::Cursor;

    /// Escape `data` into `out` with the probe's writer, returning its length.
    fn escape(data: &[u8], out: &mut [u8]) -> usize {
        let mut used = 0;
        escape_and_send(data, &mut |chunk| {
            out[used..used + chunk.len()].copy_from_slice(chunk);
            used += chunk.len();
        });
        used
    }

    /// Escape the encoded payload of `cursor`, un-escape it again and check the
    /// bytes came back unchanged; returns the decoded length.
    fn framed(cursor: &Cursor<[u8; 640]>) -> usize {
        let mut escaped = [0u8; 1280];
        let len = escape(&cursor.get_ref()[..cursor.position()], &mut escaped);
        let decoded = unescape_in_place(&mut escaped[..len]);
        assert_eq!(&escaped[..decoded], &cursor.get_ref()[..cursor.position()]);
        decoded
    }

    #[test]
    fn every_byte_survives_the_frame() {
        let mut escaped = [0u8; 2];
        for byte in 0u8..=255 {
            let len = escape(&[byte], &mut escaped);
            assert_eq!(len, if must_escape(byte) { 2 } else { 1 }, "byte {byte:#x}");
            if must_escape(byte) {
                assert_eq!(escaped[0], RPC_ESCAPE);
                assert_eq!(escaped[1], byte ^ ESCAPE_XOR);
            }
            assert!(!escaped[..len].contains(&RPC_EOM));
            assert!(!escaped[..len].contains(&RPC_START));
            let decoded = unescape_in_place(&mut escaped[..len]);
            assert_eq!(&escaped[..decoded], &[byte]);
        }
    }

    #[test]
    fn cbor_payload_with_frame_markers_survives() {
        let mut payload = [0u8; 256];
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte = index as u8;
        }
        let mut cursor = Cursor::new([0u8; 640]);
        {
            let mut encoder = Encoder::new(&mut cursor);
            let _ = encoder.array(2);
            let _ = encoder.u32(0x2300_0012);
            let _ = encoder.bytes(&payload);
        }
        let decoded = framed(&cursor);

        let mut decoder = minicbor::Decoder::new(&cursor.get_ref()[..decoded]);
        assert_eq!(decoder.array().unwrap(), Some(2));
        assert_eq!(decoder.u32().unwrap(), 0x2300_0012);
        assert_eq!(decoder.bytes().unwrap(), payload.as_slice());
    }

    #[test]
    fn long_escaped_payload_crosses_several_chunks() {
        // 1 KiB of frame markers is 2 KiB once escaped: many scratch flushes.
        let payload = [b'#'; 1024];
        let mut chunks = 0;
        let mut total = 0;
        escape_and_send(&payload, &mut |chunk| {
            chunks += 1;
            total += chunk.len();
            assert!(chunk.len() <= SCRATCH_SIZE);
            assert!(!chunk.contains(&RPC_EOM));
        });
        assert!(chunks > 1, "the scratch buffer must have been flushed");
        assert_eq!(total, 2 * payload.len());
    }

    #[test]
    fn params_reads_the_header_and_the_cbor_parameters() {
        let mut request = [0u8; 32];
        request[0] = b'G';
        request[1] = b'Z';
        let mut cursor = Cursor::new([0u8; 32]);
        {
            let mut encoder = Encoder::new(&mut cursor);
            let _ = encoder.array(3);
            let _ = encoder.u8(7);
            let _ = encoder.u16(0x1234);
            let _ = encoder.bool(true);
        }
        let len = cursor.position();
        request[2..2 + len].copy_from_slice(&cursor.get_ref()[..len]);

        let mut params = RpcParams::new(&request[..2 + len]);
        assert_eq!(params.next_cmd(), b'G');
        assert_eq!(params.next_cmd(), b'Z');
        assert!(!params.is_empty());
        assert_eq!(params.next_u8(), 7);
        assert_eq!(params.next_u16(), 0x1234);
        assert!(params.next_bool());
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn params_reads_a_trailing_block() {
        let block = [0x23u8, 0x21, 0x7d, 0x00];
        let mut request = [0u8; 32];
        request[0] = b'A';
        request[1] = b'M';
        let mut cursor = Cursor::new([0u8; 32]);
        {
            let mut encoder = Encoder::new(&mut cursor);
            let _ = encoder.array(2);
            let _ = encoder.u32(16);
            let _ = encoder.bytes(&block);
        }
        let len = cursor.position();
        request[2..2 + len].copy_from_slice(&cursor.get_ref()[..len]);

        let mut params = RpcParams::new(&request[..2 + len]);
        params.next_cmd();
        params.next_cmd();
        assert_eq!(params.next_u32(), 16);
        assert_eq!(params.end(), &block);
        assert_eq!(params.len(), 0);
    }

    #[test]
    fn malformed_input_reads_as_zero() {
        let request = *b"GZ";
        let mut params = RpcParams::new(&request);
        params.next_cmd();
        params.next_cmd();
        assert!(params.is_empty());
        assert_eq!(params.next_u32(), 0);
        assert!(!params.next_bool());
        assert_eq!(params.end(), &[]);
        // A header shorter than two bytes must not panic either.
        let mut short = RpcParams::new(b"G");
        assert_eq!(short.next_cmd(), b'G');
        assert_eq!(short.next_cmd(), 0);
    }

    #[test]
    fn dangling_escape_byte_is_kept_as_is() {
        let mut dangling = [RPC_ESCAPE];
        assert_eq!(unescape_in_place(&mut dangling), 1);
        assert_eq!(dangling[0], RPC_ESCAPE);
        let mut empty: [u8; 0] = [];
        assert_eq!(unescape_in_place(&mut empty), 0);
    }
}
