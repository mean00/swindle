// The `lnPin` encoding of the LN probes (GD32F303 / CH32V307).
//
// Bluepill compatible: `PA0..PA15 = 0..15`, `PB0..PB15 = 16..31`,
// `PC0..PC15 = 32..47` — the numbers esprit's C `lnPin` enum uses, which is also
// what `_mapping[]` in the generated `bmp_pinout_board.h` is built from.
//
// This file is compiled twice, like `board_desc.rs` (hence plain `//` comments and
// no inner doc comments): as `crate::bluepill::pins` in the crate, re-exported as
// `crate::pins` for the selected family, and inside
// `swindle/tools/gen_board_header`, which is how a board file compiles unchanged
// in both places.
//
// A wrong helper would silently move the debug pins, so the board files assert a
// few known values.

use crate::board_desc::BoardPin;

/// Port A pin.
pub(crate) const fn pa(n: i32) -> BoardPin {
    n
}

/// Port B pin.
pub(crate) const fn pb(n: i32) -> BoardPin {
    16 + n
}

/// Port C pin.
pub(crate) const fn pc(n: i32) -> BoardPin {
    32 + n
}
