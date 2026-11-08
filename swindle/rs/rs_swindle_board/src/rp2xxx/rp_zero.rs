// RP2040 / RP2350 "zero" board (CMake: USE_RP2040 or USE_RP2350, without
// `USE_RP_CARRIER`).
//
// The values are the ones in swindle/src/platform/rp2040/bmp_pinout_rp2040.h,
// which is still what the C side compiles: the board table is the source of truth
// for the code in this crate, and the two have to agree (see the assertions at the
// bottom).

use crate::board_desc::Board;
use crate::pins::gpio;

pub const BOARD: Board = Board {
    name: "rp_zero",
    dev: [
        gpio(24), // 0 TMS
        gpio(24), // 1 TDI
        gpio(24), // 2 TDO
        gpio(24), // 3 TCK
        gpio(24), // 4 TRACESWO
        gpio(12), // 5 SWDIO
        gpio(13), // 6 SWCLK
        gpio(11), // 7 RST
        gpio(10), // 8 DIRECTION (level shifter)
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

// Pin the wiring down: this is the RP zero pinout, and both the board files and
// the C header are easy to edit independently.
const _: () = {
    assert!(BOARD.dev[5] == gpio(12)); // SWDIO
    assert!(BOARD.dev[6] == gpio(13)); // SWCLK
    assert!(BOARD.dev[7] == gpio(11)); // RST
    assert!(BOARD.dev[8] == gpio(10)); // DIRECTION
};
