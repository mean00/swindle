// The `lnPin` encoding of the RP2040 / RP2350 probes.
//
// On those parts a pin *is* its GPIO number (`GPIO0` = 0 …), so the helper is the
// identity: it exists so a board file always states its encoding explicitly.
//
// This file is compiled twice, like `board_desc.rs` (hence plain `//` comments and
// no inner doc comments): as `crate::rp2xxx::pins` in the crate, re-exported as
// `crate::pins` for the selected family, and inside
// `swindle/tools/gen_board_header` when an RP board becomes renderable.

use crate::board_desc::BoardPin;

/// RP2040 / RP2350 GPIO number.
#[allow(dead_code)] // the generator never builds an RP board file (yet)
pub(crate) const fn gpio(n: i32) -> BoardPin {
    n
}
