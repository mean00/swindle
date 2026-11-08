// GD32F303 / CH32V307 probe, 48 pins package (CMake: `USE_48PIN_PACKAGE`).
//
// The debug header wiring is the same on both MCUs; only the package differs, and
// with it the ADC input the target voltage is sensed on (PA3 here, PA4 on the 64
// pins package).

use crate::board_desc::Board;
use crate::pins::{pa, pb, pc};

pub const BOARD: Board = Board {
    name: "ln_48pins",
    dev: [
        pa(0), // 0 TMS
        pa(0), // 1 TDI
        pa(0), // 2 TDO
        pa(0), // 3 TCK
        pa(0), // 4 TRACESWO
        pb(8), // 5 SWDIO
        pb(9), // 6 SWCLK
        pb(6), // 7 RST
        pc(3), // 8 DIRECTION (level shifter)
    ],
    adc_nreset: Some(pa(3)), // NRST/2 divider
    adc_multiplier: 1.0,
    // The bridge / log UART pins of this platform are owned by the esprit UART
    // driver (`rs_uart_bluepill`, instance -> pin), not by the board table.
    uart_rx: None,
    uart_tx: None,
    usb_gdb: 1,
    usb_serial: 2,
    usb_logger: 2,
    leds: &[pc(13), pa(8), pb(13)],
    ws2812: Some(pb(4)),
};

// The bluepill compatible `lnPin` encoding (see `pins.rs`): PA0..PA15 = 0..15,
// PB0..PB15 = 16..31, PC0..PC15 = 32..47. A wrong helper would silently move the
// debug pins, so pin a few down.
const _: () = {
    assert!(pb(0) == 16);
    assert!(pb(8) == 24);
    assert!(pc(3) == 35);
    assert!(BOARD.dev[5] == pb(8)); // SWDIO
    assert!(BOARD.dev[6] == pb(9)); // SWCLK
    assert!(BOARD.dev[7] == pb(6)); // RST
    assert!(BOARD.dev[8] == pc(3)); // DIRECTION
};
