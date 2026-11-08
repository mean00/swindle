//! Hosted platform serial transport wrapper.
//!
//! Provides safe Rust abstractions around Blackmagic's C platform buffer
//! I/O (`platform_buffer_read`, `platform_buffer_write_buffered`, `platform_write_flush`)
//! and the reply decoding the generated client uses (see [`crate::rpc_wire`]).

use crate::SyncCell;
use crate::rpc_common_generated::RPC_RESP_OK;
use crate::rpc_wire::unescape_in_place;
use core::ffi::c_int;

unsafe extern "C" {
    pub fn platform_buffer_read(data: *mut u8, maxsize: c_int) -> c_int;
    #[link_name = "platform_buffer_write"]
    pub fn platform_buffer_write_c(data: *const u8, size: c_int) -> c_int;
    pub fn platform_buffer_write_buffered(data: *const u8, size: c_int) -> c_int;
    pub fn platform_write_flush();
}

/// Reply buffer size. A `hex_block` reply is at most `MEM_BLOCK_MAX` bytes, each of which
/// may be escaped (twice its size), plus the CBOR head and the frame markers, so a full
/// block has to fit: the reader errors out rather than truncating.
pub const RPC_BUFFER_SIZE: usize = 2 * crate::riscv::mem::MEM_BLOCK_MAX + 64;
static RPC_BUFFER: SyncCell<[u8; RPC_BUFFER_SIZE]> = SyncCell::new([0; RPC_BUFFER_SIZE]);

/// Write data to the hosted serial buffer.
#[inline]
pub fn platform_buffer_write(data: &[u8]) {
    unsafe {
        platform_buffer_write_buffered(data.as_ptr(), data.len() as c_int);
    }
}

/// Flush the hosted serial write buffer.
#[inline]
pub fn platform_buffer_flush() {
    unsafe {
        platform_write_flush();
    }
}

/// Read an RPC reply into the static reply buffer, un-escape its CBOR payload
/// in place, and return a slice of the decoded bytes (`status` first).
pub fn remote_get_reply() -> &'static [u8] {
    unsafe {
        let buf = RPC_BUFFER.as_mut();
        let received = platform_buffer_read(buf.as_mut_ptr(), RPC_BUFFER_SIZE as c_int);
        if received <= 0 {
            return &[];
        }
        let len = unescape_in_place(&mut buf[..received as usize]);
        &buf[..len]
    }
}

// ── Reply decoding ──
// A reply is `status` `payload`, the payload being the CBOR array of the reply
// fields. Error frames (`status != 'K'`) carry the fault code instead.

/// The payload of a successful reply, `None` when the probe reported an error.
pub fn reply_payload(reply: &[u8]) -> Option<&[u8]> {
    match reply.split_first() {
        Some((&RPC_RESP_OK, payload)) => Some(payload),
        _ => None,
    }
}

/// `true` when the probe reported success (`reply = "ok"` commands).
pub fn reply_is_ok(reply: &[u8]) -> bool {
    reply_payload(reply).is_some()
}

/// A decoder positioned on the first field of a successful reply.
pub fn reply_fields(reply: &[u8]) -> Option<minicbor::Decoder<'_>> {
    let mut decoder = minicbor::Decoder::new(reply_payload(reply)?);
    decoder.array().ok()?;
    Some(decoder)
}

/// The fault code of an error frame; `1` when it carries none.
pub fn reply_fault(reply: &[u8]) -> i32 {
    let mut decoder = minicbor::Decoder::new(reply.get(1..).unwrap_or(&[]));
    let _ = decoder.array();
    decoder.i32().unwrap_or(1)
}

/// The single `u8` field of a reply (`GEN.NRST_GET`, `HL.CHECK`, …).
pub fn reply_u8(reply: &[u8]) -> Option<u8> {
    let mut decoder = reply_fields(reply)?;
    decoder.u8().ok()
}

/// The single `u32` field of a reply (`GEN.FREQ_GET`, the `SWINDLE` helpers, …).
pub fn reply_u32(reply: &[u8]) -> Option<u32> {
    let mut decoder = reply_fields(reply)?;
    decoder.u32().ok()
}

/// The byte string of a `hex_block` reply (a `mem_read` block), zero copy.
pub fn reply_bytes<'a>(reply: &'a [u8]) -> Option<&'a [u8]> {
    let mut decoder = reply_fields(reply)?;
    decoder.bytes().ok()
}

/// The text of a `reply = "string"` command (`"LNBMP"`, the target voltage).
pub fn reply_text<'a>(reply: &'a [u8]) -> Option<&'a [u8]> {
    let mut decoder = reply_fields(reply)?;
    if let Ok(text) = decoder.str() {
        return Some(text.as_bytes());
    }
    // The probe falls back to a byte string for non-UTF-8 text.
    let mut decoder = reply_fields(reply)?;
    decoder.bytes().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;
    use std::vec::Vec;
    use crate::rpc_common_generated::RPC_RESP_ERR;
    use crate::rpc_wire::escape_and_send;
    use minicbor::encode::write::Cursor;

    /// Build a reply the way the probe's `rpc_reply_encoder` does: the `&` and
    /// the `#` are stripped by the transport, so this is `status` + escaped
    /// CBOR. Anything that survives here is what the generated callers see.
    fn frame(
        status: u8,
        fill: impl FnOnce(&mut minicbor::Encoder<&mut Cursor<[u8; 256]>>),
    ) -> Vec<u8> {
        let mut cursor = Cursor::new([0u8; 256]);
        {
            let mut encoder = minicbor::Encoder::new(&mut cursor);
            fill(&mut encoder);
        }
        let mut framed = vec![status];
        escape_and_send(&cursor.get_ref()[..cursor.position()], &mut |chunk| {
            framed.extend_from_slice(chunk)
        });
        // `remote_get_reply()` un-escapes the payload before the callers see it.
        let len = unescape_in_place(&mut framed[1..]);
        framed.truncate(len + 1);
        framed
    }

    #[test]
    fn u32_reply_decodes() {
        let reply = frame(RPC_RESP_OK, |encoder| {
            let _ = encoder.array(1);
            let _ = encoder.u32(0xdead_beef);
        });
        assert!(reply_is_ok(&reply));
        assert_eq!(reply_u32(&reply), Some(0xdead_beef));
        // A value that does not fit a u8 must not be silently truncated.
        assert_eq!(reply_u8(&reply), None);
        assert_eq!(reply_bytes(&reply), None);
    }

    #[test]
    fn byte_block_reply_survives_the_frame_markers() {
        let block = [0x23u8, 0x21, 0x7d, 0x00, 0xff, 0x40];
        let reply = frame(RPC_RESP_OK, |encoder| {
            let _ = encoder.array(1);
            let _ = encoder.bytes(&block);
        });
        assert_eq!(reply_bytes(&reply), Some(&block[..]));
        assert_eq!(reply_u32(&reply), None);
    }

    #[test]
    fn error_frame_reports_the_fault_code() {
        let reply = frame(RPC_RESP_ERR, |encoder| {
            let _ = encoder.array(1);
            let _ = encoder.i32(-3);
        });
        assert!(!reply_is_ok(&reply));
        assert_eq!(reply_fault(&reply), -3);
        assert_eq!(reply_u32(&reply), None);
        // An empty reply is "something failed", not a panic.
        assert_eq!(reply_fault(&[]), 1);
        assert_eq!(reply_u32(&[]), None);
        assert!(!reply_is_ok(&[]));
    }

    #[test]
    fn text_reply_decodes_as_text_or_bytes() {
        let text = frame(RPC_RESP_OK, |encoder| {
            let _ = encoder.array(1);
            let _ = encoder.str("3.30");
        });
        assert_eq!(reply_text(&text), Some(&b"3.30"[..]));

        let bytes = frame(RPC_RESP_OK, |encoder| {
            let _ = encoder.array(1);
            let _ = encoder.bytes(b"LNBMP");
        });
        assert_eq!(reply_text(&bytes), Some(&b"LNBMP"[..]));
    }

    #[test]
    fn compound_reply_fields_are_read_in_order() {
        let reply = frame(RPC_RESP_OK, |encoder| {
            let _ = encoder.array(2);
            let _ = encoder.u32(0x1122_3344);
            let _ = encoder.u32(2);
        });
        let mut fields = reply_fields(&reply).expect("a payload");
        assert_eq!(fields.u32().unwrap(), 0x1122_3344);
        assert_eq!(fields.u32().unwrap(), 2);
    }
}
