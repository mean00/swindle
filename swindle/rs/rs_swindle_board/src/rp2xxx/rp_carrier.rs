// RP2040 / RP2350 board (CMake: USE_RP2040/USE_RP2350).
//
// The values match swindle/src/platform/rp2040/bmp_pinout_rp2040.h.

use crate::board_desc::Board;
use crate::pins::gpio;

pub const BOARD: Board = Board {
    name: "rp_carrier",
    dev: [
        gpio(24), // 0 TMS
        gpio(24), // 1 TDI
        gpio(24), // 2 TDO
        gpio(24), // 3 TCK
        gpio(24), // 4 TRACESWO
        gpio(13), // 5 SWDIO
        gpio(12), // 6 SWCLK
        gpio(10), // 7 RST
        gpio(11), // 8 DIRECTION (level shifter)
    ],
    adc_nreset: Some(gpio(26)), // NRST/2 divider
    adc_multiplier: 1.0,
    // The bridge / log UART of this platform is owned by esprit's UART driver.
    uart_rx: None,
    uart_tx: None,
    // CDC instance numbers of the C side; unused on RP2xxx so far.
    usb_gdb: 1,
    usb_serial: 2,
    usb_logger: 2,
    // No LED activity code for RP2xxx in this tree yet.
    leds: &[],
    ws2812: None,
};

// Pin the wiring down: verify that the board table matches the expected layout.
const _: () = {
    assert!(BOARD.dev[5] == gpio(13)); // SWDIO
    assert!(BOARD.dev[6] == gpio(12)); // SWCLK
    assert!(BOARD.dev[7] == gpio(10)); // RST
    assert!(BOARD.dev[8] == gpio(11)); // DIRECTION
};
