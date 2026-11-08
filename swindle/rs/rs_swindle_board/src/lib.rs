#![no_std]
#![allow(non_camel_case_types, non_snake_case)]

//! Probe board wiring — the single source of truth for the probe's pinout, ADC
//! sense input, UART / USB assignation and LEDs.
//!
//! One file per board, under its family (`ln/ln_48pins.rs`, `ln/ln_64pins.rs`,
//! `rp2xxx/rp_zero.rs`, `rp2xxx/rp_carrier.rs`), selected by
//! `swindle/board.cmake` from the CMake options and enabled here as a feature
//! (`swindle/rs/CMakeLists.txt`). The C side does not carry a second copy of these
//! numbers: the selected file is rendered as `bmp_pinout_board.h` by
//! `swindle/tools/gen_board_header`, so `_mapping[]`, `PIN_ADC_NRESET_*` and the
//! `LN_*_INSTANCE` defines the C code sees are all generated from this table.
//!
//! Adding a board: write the file in its family directory, add its feature to
//! `Cargo.toml`, and map a CMake option set to it in `swindle/board.cmake` (plus the
//! matching pass-through feature in `rs_swindle` / `rs_swindle_leds` when they need it).

pub mod board_desc;

#[cfg(feature = "ln48")]
#[path = "bluepill/ln_48pins.rs"]
mod board;

#[cfg(feature = "ln64")]
#[path = "bluepill/ln_64pins.rs"]
mod board;

#[cfg(feature = "rp_zero")]
#[path = "rp2xxx/rp_zero.rs"]
mod board;

#[cfg(feature = "rp_carrier")]
#[path = "rp2xxx/rp_carrier.rs"]
mod board;

#[cfg(not(any(
    feature = "ln48",
    feature = "ln64",
    feature = "rp_zero",
    feature = "rp_carrier"
)))]
compile_error!(
    "exactly one board feature must be enabled (ln48, ln64, rp_zero, rp_carrier) — see swindle/board.cmake"
);

/// LN target code (GD32F303 / CH32V307): how this board drives its own signals, and
/// the pin encoding its tables are written with.
#[cfg(any(feature = "ln48", feature = "ln64"))]
pub mod bluepill;

/// RP2040 / RP2350 target code, same idea.
#[cfg(any(feature = "rp_zero", feature = "rp_carrier"))]
pub mod rp2xxx;

/// The pin encoding of the selected family, under one name: a board file writes
/// `crate::pins::pa(3)` (bluepill) or `crate::pins::gpio(11)` (RP) and compiles
/// the same way in the crate and in `swindle/tools/gen_board_header`, which
/// supplies this module from `SWINDLE_BOARD_PINS_RS`.
#[cfg(any(feature = "ln48", feature = "ln64"))]
pub(crate) use bluepill::pins;
#[cfg(any(feature = "rp_zero", feature = "rp_carrier"))]
pub(crate) use rp2xxx::pins;

/// The selected board's own code under one path, whatever the target is: this is
/// what `rs_swindle::reset_pin` drives.
#[cfg(any(feature = "ln48", feature = "ln64"))]
pub use bluepill::reset_pin;
#[cfg(any(feature = "rp_zero", feature = "rp_carrier"))]
pub use rp2xxx::reset_pin;

pub use board::BOARD;
