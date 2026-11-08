// Description of a probe board: the pins, the ADC sense input, the UART / USB
// assignation and the LEDs, with no knowledge of any particular MCU family.
//
// This file is compiled twice: as `rs_swindle_board::board_desc` in the firmware,
// and at the root of `swindle/tools/gen_board_header`, which renders the selected
// board file as `bmp_pinout_board.h`. Both `include!` it, so it must stay free
// of hardware access (no esprit, no `no_std`-only tricks) and the board files must
// only use the names it exports plus their family's pin encodings (`crate::pins`,
// which the generator supplies as `SWINDLE_BOARD_PINS_RS`). (Plain `//` comments,
// not `//!`: an `include!` only accepts items, so inner doc comments do not compile.)

/// One GPIO number, in the encoding the platform's `lnPin` enum uses.
///
/// This is not a portable number: `PA3` is 3 on the bluepill compatible parts,
/// `GPIO13` is 13 on the RP2040/RP2350/ESP32 ones. A board file never writes a bare
/// number: it goes through its family's helpers (`ln::pins::pa/pb/pc`,
/// `rp2xxx::pins::gpio`), so each encoding is written down once.
pub type BoardPin = i32;

/// Everything the firmware knows about the probe board's wiring.
pub struct Board {
    /// Board file name, e.g. `ln_48pins`.
    pub name: &'static str,
    /// The nine logical debug signals, in `lnBMPPins` order
    /// (TMS, TDI, TDO, TCK, TRACESWO, SWDIO, SWCLK, RST, DIRECTION).
    pub dev: [BoardPin; 9],
    /// ADC input the target voltage is measured on (`None`: no voltage sensing).
    pub adc_nreset: Option<BoardPin>,
    /// Scale applied to that measurement (2.0 when the divider is /2).
    pub adc_multiplier: f32,
    /// UART the USB <-> target bridge drives, when the board wires one.
    pub uart_rx: Option<BoardPin>,
    pub uart_tx: Option<BoardPin>,
    /// CDC instances: GDB, UART bridge, logger.
    pub usb_gdb: u32,
    pub usb_serial: u32,
    pub usb_logger: u32,
    /// Activity LEDs (open drain, active low).
    pub leds: &'static [BoardPin],
    /// WS2812B data pin, when the board has one.
    pub ws2812: Option<BoardPin>,
}

/// Index of the target nRST line in [`Board::dev`], i.e. `lnBMPPins::TRESET_PIN`.
///
/// The order of `dev` is the C enum's (`swindle/private_include/lnBMP_pins.h`),
/// which this crate cannot see, so the index is named here instead of being
/// repeated as a bare `7` by the code that drives the line.
///
/// `dead_code` because this file is also `include!`d by the header generator
/// (`swindle/tools/gen_board_header`), which renders all of `dev` and has no use
/// for a single index.
#[allow(dead_code)]
pub const DEV_NRST: usize = 7;
