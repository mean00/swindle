//! Probe-side RPC replies: one CBOR array per reply, escaped into the frame.
//!
//! The frame is `&` `<status>` `<escaped cbor>` `#` (see [`crate::rpc_wire`]).
//! The payload is written straight into the transport by [`rpc_reply_encoder`],
//! so a 1 KiB memory block costs neither a second buffer nor a copy.
//!
//! A reply with no field (`reply = "ok"`) carries no payload at all.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use crate::SyncCell;
use crate::encoder::encoder;
use crate::rpc_common::*;
use crate::rpc_wire::{EscapingWriter, RPC_EOM};

// DF -- not thread safe, not re-entrant, ugly but simple
// Large enough for the ADIV5 `mem_read` block; `hex_block` replies are streamed
// out as they are encoded, so nothing else needs a buffer this size.
pub(crate) const REPLY_TEMP_BUFFER_SIZE: usize = 1024;
#[repr(align(4))]
struct AlignedBuffer([u8; REPLY_TEMP_BUFFER_SIZE]);
static TEMP_BUFFER: SyncCell<AlignedBuffer> =
    SyncCell::new(AlignedBuffer([0; REPLY_TEMP_BUFFER_SIZE]));

setup_log!(false);

/// The transport write, as a plain function pointer, so [`rpc_reply_encoder`]
/// stays one concrete type instead of a family of monomorphisations.
type SendFn = fn(&[u8]);

pub(crate) fn get_temp_buffer() -> &'static mut [u8] {
    unsafe { &mut TEMP_BUFFER.as_mut().0 }
}

/// Writes one reply: the `&`/status preamble in `new()`, the CBOR payload
/// through the methods, the frame terminator in `end()`.
pub struct rpc_reply_encoder {
    cbor: minicbor::Encoder<EscapingWriter<SendFn>>,
}

pub type RpcReplyEncoder = rpc_reply_encoder;

impl Default for rpc_reply_encoder {
    fn default() -> Self {
        Self::new()
    }
}

impl rpc_reply_encoder {
    pub fn new() -> Self {
        Self::new_raw(RPC_RESP_OK)
    }

    pub fn new_error() -> Self {
        Self::new_raw(RPC_RESP_ERR)
    }

    fn new_raw(status: u8) -> Self {
        encoder::raw_send_u8(&[RPC_REMOTE_RESP, status]);
        rpc_reply_encoder {
            cbor: minicbor::Encoder::new(EscapingWriter(encoder::raw_send_u8 as SendFn)),
        }
    }

    /// Open the payload array: one element per reply field.
    pub fn array(&mut self, len: u64) {
        let _ = self.cbor.array(len);
    }

    pub fn u8(&mut self, value: u8) {
        let _ = self.cbor.u8(value);
    }

    pub fn u32(&mut self, value: u32) {
        let _ = self.cbor.u32(value);
    }

    pub fn i32(&mut self, value: i32) {
        let _ = self.cbor.i32(value);
    }

    pub fn bool(&mut self, value: bool) {
        let _ = self.cbor.bool(value);
    }

    pub fn bytes(&mut self, value: &[u8]) {
        let _ = self.cbor.bytes(value);
    }

    /// A text (CBOR `tstr`) element — the `reply = "string"` commands. Falls
    /// back to a byte string if the text is not valid UTF-8.
    pub fn text(&mut self, value: &[u8]) {
        match core::str::from_utf8(value) {
            Ok(text) => {
                let _ = self.cbor.str(text);
            }
            Err(_) => self.bytes(value),
        }
    }

    /// Close the frame and flush it.
    pub fn end(&mut self) {
        encoder::raw_send_u8(&[RPC_EOM]);
        encoder::flush();
    }
}

// ── Reply helpers used by the generated dispatch code ──

/// `reply = "ok"`: status only, no payload.
#[inline]
pub fn rpc_reply_ok() {
    rpc_reply_encoder::new().end();
}

/// A single `u8` reply (`GEN.NRST_GET`, `HL.CHECK`, …).
pub fn rpc_reply_u8(value: u8) {
    let mut reply = rpc_reply_encoder::new();
    reply.array(1);
    reply.u8(value);
    reply.end();
}

/// A single `u32` reply (`GEN.FREQ_GET`, the `SWINDLE` helpers, …).
pub fn rpc_reply_u32(value: u32) {
    let mut reply = rpc_reply_encoder::new();
    reply.array(1);
    reply.u32(value);
    reply.end();
}

/// A single byte-string reply (`hex_block`, i.e. a `mem_read` block).
pub fn rpc_reply_bytes(data: &[u8]) {
    let mut reply = rpc_reply_encoder::new();
    reply.array(1);
    reply.bytes(data);
    reply.end();
}

/// A single text reply (`GEN.START`, `GEN.VOLTAGE`).
#[unsafe(no_mangle)]
pub fn rpc_reply_string(text: &[u8]) {
    let mut reply = rpc_reply_encoder::new();
    reply.array(1);
    reply.text(text);
    reply.end();
}

/// An error frame carrying the code the host reports as the fault.
pub fn rpc_reply_error(code: i32) {
    let mut reply = rpc_reply_encoder::new_error();
    reply.array(1);
    reply.i32(code);
    reply.end();
}

/// `on_error = "ok_bool"`: `(true, value)`, or an error frame giving `false`.
pub fn rpc_reply_bool_u32(ok: bool, value: u32) {
    if ok {
        rpc_reply_u32(value);
    } else {
        rpc_reply_error(1);
    }
}

/// `on_error = "fault"` with a `u32` reply: a fault becomes an error frame.
pub fn rpc_reply_fault_u32(fault: i32, value: u32) {
    if fault != 0 {
        rpc_reply_error(fault);
    } else {
        rpc_reply_u32(value);
    }
}

/// `on_error = "fault"` with a `hex_block` reply.
pub fn rpc_reply_fault_bytes(fault: i32, data: &[u8]) {
    if fault != 0 {
        rpc_reply_error(fault);
    } else {
        rpc_reply_bytes(data);
    }
}
