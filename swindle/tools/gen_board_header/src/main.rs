//! Render the probe board table of `rs_swindle_board` as `bmp_pinout_board.h`.
//!
//! Compiled and run by `swindle/board.cmake`, once per build, after the board has
//! been selected from the CMake options. The C side has no pin table of its own any
//! more: `bmp_pinout.h` includes this header, so `_mapping[]`, `PIN_ADC_NRESET_*`,
//! `LN_UART_*` and the `LN_*_INSTANCE` defines stay compile-time constants while
//! their values live in `swindle/rs/rs_swindle_board/src/<board>.rs`.
//!
//! The files to include are passed in the environment at compile time (a
//! `include!` path has to be known while compiling):
//!   SWINDLE_BOARD_DESC_RS : .../rs_swindle_board/src/board_desc.rs
//!   SWINDLE_BOARD_PINS_RS : .../rs_swindle_board/src/<family>/pins.rs
//!   SWINDLE_BOARD_RS      : .../rs_swindle_board/src/<selected board>.rs
//!
//! The three wrappers below are what a board file sees as `crate::board_desc`,
//! `crate::pins` and (implicitly, it is the module it lives in) its own items — the
//! same names the crate provides, which is why a board file compiles unchanged in
//! both places.
//!
//! Usage: `gen_board_header [-o <header path>]` (stdout when `-o` is absent).

mod board_desc {
    include!(concat!(env!("SWINDLE_BOARD_DESC_RS")));
}

mod pins {
    include!(concat!(env!("SWINDLE_BOARD_PINS_RS")));
}

mod board {
    include!(concat!(env!("SWINDLE_BOARD_RS")));
}

use std::fmt::Write as _;

/// Names of the nine logical debug signals, in `lnBMPPins` order.
const DEV_PIN_NAMES: [&str; 9] = ["TMS", "TDI", "TDO", "TCK", "TRACESWO", "SWDIO", "SWCLK", "RST", "DIRECTION"];

fn main() {
    let board = &board::BOARD;

    let mut out = String::new();
    let _ = writeln!(out, "/**");
    let _ = writeln!(out, " * @file bmp_pinout_board.h");
    let _ = writeln!(out, " * @brief Probe board wiring for `{}` — GENERATED FILE, do not edit.", board.name);
    let _ = writeln!(out, " *");
    let _ = writeln!(out, " * Rendered from swindle/rs/rs_swindle_board/src/{}.rs by", board.name);
    let _ = writeln!(out, " * swindle/tools/gen_board_header (swindle/board.cmake). Change the board");
    let _ = writeln!(out, " * table there, not this header.");
    let _ = writeln!(out, " */");
    let _ = writeln!(out);
    let _ = writeln!(out, "#pragma once");
    let _ = writeln!(out);
    let _ = writeln!(out, "#include \"lnGPIO.h\"");
    let _ = writeln!(out);
    let _ = writeln!(out, "// Logical debug signals, in lnBMPPins order (see lnBMP_pins.h).");
    let _ = writeln!(out, "const lnPin _mapping[9] = {{");
    for (i, pin) in board.dev.iter().enumerate() {
        let _ = writeln!(out, "    (lnPin){}, // {} {}", pin, i, DEV_PIN_NAMES[i]);
    }
    let _ = writeln!(out, "}};");
    let _ = writeln!(out);

    // Target voltage sense. Without a pin the C side keeps its "no ADC" path
    // (platform_get_adc_nreset_pin() returns 0), exactly as an undefined macro did.
    let _ = writeln!(out, "// Target voltage sense (NRST/2 divider).");
    match board.adc_nreset {
        Some(pin) => {
            let _ = writeln!(out, "#define PIN_ADC_NRESET_DIV_BY_TWO ((lnPin){})", pin);
        }
        None => {
            let _ = writeln!(out, "// No voltage sensing on this board (PIN_ADC_NRESET_DIV_BY_TWO is undefined).");
        }
    }
    let _ = writeln!(out, "#define PIN_ADC_NRESET_MULTIPLIER {:.1}f", board.adc_multiplier);
    let _ = writeln!(out);

    // UART of the USB <-> target bridge. Only the boards that wire one say so; on
    // the bluepill compatible parts the esprit UART driver owns the pins.
    match (board.uart_rx, board.uart_tx) {
        (Some(rx), Some(tx)) => {
            let _ = writeln!(out, "// UART of the USB <-> target bridge.");
            let _ = writeln!(out, "#define LN_UART_RX ((lnPin){})", rx);
            let _ = writeln!(out, "#define LN_UART_TX ((lnPin){})", tx);
            let _ = writeln!(out);
        }
        _ => {}
    }

    let _ = writeln!(out, "// USB CDC instances of the probe.");
    let _ = writeln!(out, "#define LN_USB_INSTANCE {}", board.usb_gdb);
    let _ = writeln!(out, "#define LN_SERIAL_INSTANCE {}", board.usb_serial);
    let _ = writeln!(out, "#define LN_LOGGER_INSTANCE {}", board.usb_logger);
    let _ = writeln!(out);

    let _ = write!(out, "// Activity LEDs (open drain, active low):");
    for pin in board.leds {
        let _ = write!(out, " {}", pin);
    }
    let _ = writeln!(out);
    let _ = write!(out, "#define LN_BOARD_LEDS {{");
    for (i, pin) in board.leds.iter().enumerate() {
        let _ = write!(out, "{}(lnPin){}", if i == 0 { "" } else { ", " }, pin);
    }
    let _ = writeln!(out, " }}");
    if let Some(pin) = board.ws2812 {
        let _ = writeln!(out, "#define LN_WS2812_PIN ((lnPin){})", pin);
    }

    let args: Vec<String> = std::env::args().collect();
    match args.iter().position(|a| a == "-o") {
        Some(i) => {
            let path = match args.get(i + 1) {
                Some(p) => p,
                None => {
                    eprintln!("gen_board_header: -o needs a path");
                    std::process::exit(2);
                }
            };
            if let Err(e) = std::fs::write(path, out) {
                eprintln!("gen_board_header: cannot write {}: {}", path, e);
                std::process::exit(1);
            }
        }
        None => print!("{}", out),
    }
}
