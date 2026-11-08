//! LN probes (GD32F303 / CH32V307) — the "bluepill" family: the wiring of the two
//! packages, the pin encoding the tables are written with, and the code that drives
//! the board's own signals.
//!
//! The pin tables stay plain data (`ln_48pins.rs` / `ln_64pins.rs`, selected in
//! `lib.rs`) because `swindle/tools/gen_board_header` `include!`s them; everything
//! else of the target lives here and is part of the crate.

pub(crate) mod pins;
pub mod reset_pin;
