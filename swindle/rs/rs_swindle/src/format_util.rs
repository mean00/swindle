//! Hex formatting — the inverse of [`crate::parsing_util`].
//!
//! Target drivers have to reproduce the C drivers' `printf` output exactly, and
//! those mix both letter cases: `stm32_common.c` renders a UID with
//! `%08" PRIX32` (upper) while `stm32f4.c`'s `mon option` dump uses `%08" PRIx32`
//! (lower). `ufmt` supports no width or fill, so the digits are rendered by hand —
//! here, once, for every caller.

/// Letter case of the digits a caller prints.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HexCase {
    /// `0..9a..f`, as `%x` / `PRIx32` print them.
    Lower,
    /// `0..9A..F`, as `%X` / `PRIX32` print them.
    Upper,
}

const HEX_DIGITS_LOWER: &[u8; 16] = b"0123456789abcdef";
const HEX_DIGITS_UPPER: &[u8; 16] = b"0123456789ABCDEF";

/// Widest hex value a `u32` needs, and the size of a [`hex_digits`] buffer.
pub const MAX_HEX_DIGITS: usize = 8;

/// Render the low `digits` nibbles of `value` into `out`, most significant first,
/// and borrow the filled prefix of `out`.
///
/// `digits` is clamped to `1..=MAX_HEX_DIGITS` and to `out.len()`, so this never
/// panics: on a target a `panic!` is an abort, not an error message.
pub fn hex_digits(value: u32, digits: usize, case: HexCase, out: &mut [u8]) -> &str {
    let table = match case {
        HexCase::Lower => HEX_DIGITS_LOWER,
        HexCase::Upper => HEX_DIGITS_UPPER,
    };
    let digits = digits.clamp(1, MAX_HEX_DIGITS).min(out.len());
    for (index, digit) in out[..digits].iter_mut().enumerate() {
        let shift = 4 * (digits - 1 - index);
        *digit = table[((value >> shift) & 0xf) as usize];
    }
    // Only hex digits were written, so the bytes are valid UTF-8.
    unsafe { core::str::from_utf8_unchecked(&out[..digits]) }
}

/// The `0x` that [`hex_padded`] puts in front of the digits.
pub const HEX_PREFIX: &[u8] = b"0x";

/// Size of a [`hex_padded`] buffer: the prefix plus the widest value.
pub const HEX_BUFFER_SIZE: usize = HEX_PREFIX.len() + MAX_HEX_DIGITS;

/// Render `0x` followed by the low `digits` nibbles of `value` into `out`, as one string.
///
/// `digits` is clamped as in [`hex_digits`]. The prefix and the digits have to leave as a
/// single string: every sink forwards one write as one chunk, and a chunk boundary inside a
/// number is what splits a log line in two. `out` should be [`HEX_BUFFER_SIZE`]; anything
/// shorter than the prefix plus one digit yields `""`.
pub fn hex_padded(value: u32, digits: usize, case: HexCase, out: &mut [u8]) -> &str {
    if out.len() < HEX_PREFIX.len() + 1 {
        return "";
    }
    let (prefix, rest) = out.split_at_mut(HEX_PREFIX.len());
    prefix.copy_from_slice(HEX_PREFIX);
    // `.len()` ends the mutable borrow of `rest` before `out` is read below.
    let len = hex_digits(value, digits, case, rest).len();
    // Only the prefix and hex digits were written, so the bytes are valid UTF-8.
    unsafe { core::str::from_utf8_unchecked(&out[..HEX_PREFIX.len() + len]) }
}

/// Digits [`hex_digits`] needs to print `value` without leading zeros (at least one).
pub fn hex_width(value: u32) -> usize {
    // Each digit covers one nibble; a zero still needs one digit.
    let leading_nibbles = (value.leading_zeros() / 4) as usize;
    (MAX_HEX_DIGITS - leading_nibbles).clamp(1, MAX_HEX_DIGITS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padded_is_prefix_and_digits_in_one_string() {
        let mut buf = [0u8; HEX_BUFFER_SIZE];
        assert_eq!(hex_padded(0x1a2b, 4, HexCase::Lower, &mut buf), "0x1a2b");
        assert_eq!(hex_padded(0x1a2b, 8, HexCase::Upper, &mut buf), "0x00001A2B");
        assert_eq!(hex_padded(0, 1, HexCase::Lower, &mut buf), "0x0");
        assert_eq!(hex_padded(u32::MAX, 8, HexCase::Lower, &mut buf), "0xffffffff");
    }

    #[test]
    fn padded_clamps_digits_like_hex_digits() {
        let mut buf = [0u8; HEX_BUFFER_SIZE];
        // Zero digits and more digits than a u32 holds both clamp instead of panicking.
        assert_eq!(hex_padded(0xab, 0, HexCase::Lower, &mut buf), "0xb");
        assert_eq!(hex_padded(0xab, 99, HexCase::Lower, &mut buf), "0x000000ab");
    }

    #[test]
    fn padded_returns_empty_for_a_buffer_shorter_than_the_prefix() {
        let mut tiny = [0u8; 2];
        assert_eq!(hex_padded(0x1234, 8, HexCase::Lower, &mut tiny), "");
    }
}
