//! WS2812B LED breathing on RP2040 / RP2350 via PIO1 SM0.

use core::mem::offset_of;
use core::sync::atomic::{AtomicU32, Ordering};

use rs_rp2xxx::common::PIO1_BASE;
use rust_esprit::delay_ms;
use rust_esprit::rp2xxx::rcu::{Peripheral, get_clock};

use crate::sine_table::XSIN;

/// GPIO pin wired to the WS2812B data line (GPIO16 on Zero, GPIO23 on full-size).
#[cfg(feature = "rp_zero")]
const WS2812_PIN: u32 = 16;
#[cfg(not(feature = "rp_zero"))]
const WS2812_PIN: u32 = 23;

/// PIO engine used for WS2812B (`LN_WS2812_PIO_ENGINE`).
const PIO_INSTANCE: i32 = 1;

/// Target PIO state machine clock in Hz (`800 kHz * 10` cycles/bit).
const WS_SPEED_HZ: u32 = 8_000_000;

/// Breathing loop step and delay in ms (matches `xsin_table.h` and `swindle_common_rp2040.cpp`).
const BREATHING_STEP: usize = 1;
const BREATHING_DELAY_MS: u32 = 6;
const SINE_LAST_INDEX: usize = XSIN.len() - 1;

/// Bit shift into the 24-bit GRB word (0 = red, 16 = green when connected).
const SHIFT_DISCONNECTED: u32 = 0;
const SHIFT_CONNECTED: u32 = 16;

/// `ws.pio` assembled instructions (`pioasm`).
const WS_PROGRAM: [u16; 9] = [
    0x80a0, // 0: pull   block
    0xe057, // 1: set    y, 23
    0xe000, // 2: set    pins, 0
    0x6028, // 3: out    x, 8
    0xe101, // 4: set    pins, 1 [1]
    0x6021, // 5: out    x, 1
    0xa401, // 6: mov    pins, x [4]
    0xe100, // 7: set    pins, 0 [1]
    0x0084, // 8: jmp    y--, 4
];
const WS_WRAP_TARGET: u32 = 0;
const WS_WRAP: u32 = 8;

/// `SET PINDIRS, 1` PIO instruction (`opcode=7 << 13 | dest=4 << 5 | data=1`).
const PIO_INSTR_SET_PINDIR_OUT: u32 = (7 << 13) | (4 << 5) | 1;

const PIO_CTRL_SM0_ENABLE: u32 = 1 << 0;
const PIO_CTRL_SM0_RESTART: u32 = 1 << 4;
const PIO_CTRL_SM0_CLKDIV_RESTART: u32 = 1 << 8;
const PIO_FSTAT_TX_FULL_SM0: u32 = 1 << 16;
const PIO_SHIFTCTRL_IN_SHIFTDIR: u32 = 1 << 18;

#[repr(C)]
struct PioSmRegisters {
    clkdiv: u32,
    execctrl: u32,
    shiftctrl: u32,
    addr: u32,
    instr: u32,
    pinctrl: u32,
}

#[repr(C)]
struct PioRegisters {
    ctrl: u32,               // 0x00
    fstat: u32,              // 0x04
    fdebug: u32,             // 0x08
    flevel: u32,             // 0x0C
    txf: [u32; 4],           // 0x10
    rxf: [u32; 4],           // 0x20
    irq: u32,                // 0x30
    irq_force: u32,          // 0x34
    input_sync_bypass: u32,  // 0x38
    dbg_padout: u32,         // 0x3C
    dbg_padoe: u32,          // 0x40
    dbg_cfginfo: u32,        // 0x44
    instr_mem: [u32; 32],    // 0x48
    sm: [PioSmRegisters; 4], // 0xC8
}

const _: () = assert!(offset_of!(PioRegisters, instr_mem) == 0x48);
const _: () = assert!(offset_of!(PioRegisters, sm) == 0xC8);

macro_rules! pio_reg {
    ($member:ident) => {
        PIO1_BASE + offset_of!(PioRegisters, $member) as u32
    };
}

macro_rules! pio_sm0_reg {
    ($member:ident) => {
        PIO1_BASE + offset_of!(PioRegisters, sm) as u32 + offset_of!(PioSmRegisters, $member) as u32
    };
}

#[inline(always)]
fn read_reg(addr: u32) -> u32 {
    // SAFETY: `addr` is a documented PIO1 register offset from `PIO1_BASE`.
    unsafe { core::ptr::read_volatile(addr as *const u32) }
}

#[inline(always)]
fn write_reg(addr: u32, val: u32) {
    // SAFETY: `addr` is a documented PIO1 register offset from `PIO1_BASE`.
    unsafe { core::ptr::write_volatile(addr as *mut u32, val) }
}

unsafe extern "C" {
    #[link_name = "\u{1}_Z12lnPinModePIO5lnPinib"]
    fn ln_pin_mode_pio(pin: i32, instance: i32, pullup: bool);
}

static COLOR_SHIFT: AtomicU32 = AtomicU32::new(SHIFT_DISCONNECTED);

/// Switch the breathing LED color between disconnected (red) and connected (green).
#[unsafe(no_mangle)]
pub extern "C" fn swindle_leds_set_connected(connected: bool) {
    let shift = if connected {
        SHIFT_CONNECTED
    } else {
        SHIFT_DISCONNECTED
    };
    COLOR_SHIFT.store(shift, Ordering::Relaxed);
}

fn pio_ws2812_init(pin: u32) {
    // SAFETY: routes `pin` to PIO1 as an output pad.
    unsafe { ln_pin_mode_pio(pin as i32, PIO_INSTANCE, false) };

    // Disable SM0 and restart its state + clock divider.
    let ctrl = (read_reg(pio_reg!(ctrl)) & !PIO_CTRL_SM0_ENABLE)
        | PIO_CTRL_SM0_RESTART
        | PIO_CTRL_SM0_CLKDIV_RESTART;
    write_reg(pio_reg!(ctrl), ctrl);

    // Upload the 9-instruction WS2812 program into PIO1 instruction memory.
    let imem_base = pio_reg!(instr_mem);
    for (i, &insn) in WS_PROGRAM.iter().enumerate() {
        write_reg(imem_base + (i as u32) * 4, insn as u32);
    }

    // Configure wrap, clock divider, and shift direction (MSB first on OUT).
    write_reg(
        pio_sm0_reg!(execctrl),
        (WS_WRAP << 12) | (WS_WRAP_TARGET << 7),
    );
    let sys_hz = get_clock(Peripheral::SysClock);
    let intdiv = (sys_hz / WS_SPEED_HZ).max(1);
    write_reg(pio_sm0_reg!(clkdiv), intdiv << 16);

    // Set pin direction to output via an immediate `SET PINDIRS, 1` instruction.
    write_reg(pio_sm0_reg!(shiftctrl), 0);
    write_reg(pio_sm0_reg!(pinctrl), (1 << 26) | (pin << 5));
    write_reg(pio_sm0_reg!(instr), PIO_INSTR_SET_PINDIR_OUT);

    // Restore shift direction and program OUT/SET pin mapping.
    write_reg(pio_sm0_reg!(shiftctrl), PIO_SHIFTCTRL_IN_SHIFTDIR);
    let pinctrl = (1 << 26) | (1 << 20) | (pin << 5) | pin;
    write_reg(pio_sm0_reg!(pinctrl), pinctrl);

    // Enable SM0.
    let ctrl = read_reg(pio_reg!(ctrl))
        | PIO_CTRL_SM0_ENABLE
        | PIO_CTRL_SM0_RESTART
        | PIO_CTRL_SM0_CLKDIV_RESTART;
    write_reg(pio_reg!(ctrl), ctrl);
}

/// Run the WS2812B breathing loop on RP2040 / RP2350. Never returns.
pub fn run() -> ! {
    pio_ws2812_init(WS2812_PIN);

    let mut pix: usize = 0;
    let mut rising = true;
    loop {
        let shift = COLOR_SHIFT.load(Ordering::Relaxed);
        let word = ((XSIN[pix] >> 4) as u32) << shift;
        if (read_reg(pio_reg!(fstat)) & PIO_FSTAT_TX_FULL_SM0) == 0 {
            write_reg(pio_reg!(txf), word);
        }
        if rising {
            pix += BREATHING_STEP;
            if pix > SINE_LAST_INDEX - BREATHING_STEP {
                rising = false;
            }
        } else {
            pix -= BREATHING_STEP;
            if pix == 0 {
                rising = true;
            }
        }
        delay_ms(BREATHING_DELAY_MS);
    }
}
