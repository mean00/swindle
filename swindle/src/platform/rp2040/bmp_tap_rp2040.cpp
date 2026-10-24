/**
 * @file bmp_tap_rp2040.cpp
 * @brief SWD TAP initialisation and PIO-based clock control (RP2040)
 */

/*
  lnBMP: Gpio driver for SWD
  This code is derived from the blackmagic one but has been modified
  to aim at simplicity at the expense of performances (does not matter much though)
  (The compiler may mitigate that by inlining)

Original license header

 * This file is part of the Black Magic Debug project.
 *
 * Copyright (C) 2011  Black Sphere Technologies Ltd.
 * Written by Gareth McMullin <gareth@blacksphere.co.nz>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.


 This file implements the SW-DP interface.

 */
#include "esprit.h"
extern "C"
{
#include "adiv5.h"
#include "general.h"
#include "timing.h"
}

#include "bmp_pinout.h"
#include "lnBMP_pins.h"
#include "ln_rp_pio.h"
// clang-format on
#include "bmp_pinmode.h"
#include "lnRP2040_pio.h"
extern "C"
{
#include "ln_rp_clocks.h"
}
#include "bmp_pio_rvswd.h"
#include "bmp_pio_swd.h"
#include "lnBMP_reset.h"
#include "platform_support.h"

#include <new> // placement new for the .bss rpPIO instance
/* SDI pin hand-over (sdi_pinmode_enter/leave): the one thing this file needs from
 * the optional WCH SDI support, whose PIO upload (bmp_pio_sdi.h) lives in
 * bmp_tap_rp2040_riscv_extra.cpp. Nothing else in this file is SDI. */
#include "bmp_riscv_extra.h"

extern void gmp_gpio_init_adc();

static bmp_pin_mode currentPioMode;
uint32_t swd_frequency = 1000 * 1000; // 1Mhz by default
rpPIO *swdpio = NULL;
rpPIO_SM *xsm = NULL;
SwdReset *pReset;
/* PIO instructions per bit clock, per wire program. The state machine is clocked at
 * swd_frequency * <factor>, so a program spending N instructions per bit ends up at
 * swd_frequency * factor / N:
 *   rvswd.pio / rvswd_raw : 5 instructions per bit -> factor 5 -> the bit clock is
 *                           exactly swd_frequency, i.e. `mon fq` *is* the wire clock.
 *   swd.pio               : 6 instructions per bit, kept at the historical factor 4
 *                           (its clock stays below the requested rate; changing it
 *                           would alter a known-good SWD path).
 */
#define SWD_PIO_RATE_FACTOR 4
#define RVSWD_PIO_RATE_FACTOR 5

/**
 * @brief State-machine rate factor (a multiple of swd_frequency) for a wire mode.
 * @param mode The active pin mode.
 * @return uint32_t The factor to multiply swd_frequency by.
 */
static uint32_t pioRateFactor(bmp_pin_mode mode)
{
    switch (mode)
    {
    case BMP_PINMODE_RVSWD:
    case BMP_PINMODE_RVSWD_RAW:
        return RVSWD_PIO_RATE_FACTOR;
    default:
        return SWD_PIO_RATE_FACTOR;
    }
}

/**
 * @brief Set the RP2040 PIO state machine clock frequency for SWD.
 * @param fq Desired frequency in Hz.
 */
static void rp2040_swd_pio_change_clock(uint32_t fq)
{
    xsm->stop();
    xsm->setSpeed(fq);
    xsm->execute();
}

/**
 * @brief Set the SWD frequency (public API).
 * @param fq Desired frequency in Hz.
 */
extern "C" void bmp_set_frequency_c(uint32_t fq)
{
    swd_frequency = fq;
    /* Keep the wire clock at fq for the mode we are in; the next pinmode switch
     * re-applies it through setupPIO() anyway. */
    rp2040_swd_pio_change_clock(fq * pioRateFactor(currentPioMode));
}
/**
 * @brief One-time init of GPIO, PIO engine, ADC, and reset pin.
 */
void bmp_gpio_init_once()
{
    /* One instance of each, for the lifetime of the probe: construct them on the
     * first call (same ordering as the old `new`) but keep them in .bss.
     * rpPIO has a virtual destructor, so a plain function-local static would drag
     * newlib's exit machinery in (__cxa_atexit + on_exits, 384 B) for a destructor
     * nobody calls -> placement new into .bss storage instead. */
    static SwdReset reset(TRESET_PIN); // automatically add delay after toggle
    alignas(rpPIO) static uint8_t pio_storage[sizeof(rpPIO)];
    pReset = &reset;
    pReset->setup();
    swdpio = new (pio_storage) rpPIO(LN_SWD_PIO_ENGINE);
    xsm = swdpio->getSm(0);
    currentPioMode = BMP_PINMODE_NONE;
    gmp_gpio_init_adc();
}
/**
 * @brief Upload and configure PIO program for SWD/RVSWD.
 * @param prgSizeInHalfWord Program size in half-words.
 * @param prg               PIO instruction array.
 * @param inputRight        Bit shift direction for input (true=LSB).
 * @param outputRight       Bit shift direction for output (true=LSB).
 * @param wrapTarget        Wrap target address.
 * @param wrap              Wrap address.
 * @param rateFactor        State-machine rate in multiples of swd_frequency, so the
 *                          wire clock of the program is what the user asked for.
 */
static void setupPIO(int prgSizeInHalfWord, const uint16_t *prg, bool inputRight, bool outputRight, int wrapTarget,
                     int wrap, uint32_t rateFactor)
{
    lnPin pin_swd = _mapping[TSWDIO_PIN];
    lnPin pin_clk = _mapping[TSWDCK_PIN];
    lnPin pin_direction = _mapping[TDIRECTION_PIN];
    xsm->reset();
    rpPIO_pinConfig pinConfig;
    pinConfig.sets.pinNb = 1;
    pinConfig.sets.startPin = pin_swd;
    pinConfig.outputs.pinNb = 1;
    pinConfig.outputs.startPin = pin_swd;
    pinConfig.inputs.pinNb = 1;
    pinConfig.inputs.startPin = pin_swd;
#if 0
    xsm->setSpeed(10);
#else
    xsm->setSpeed(swd_frequency * rateFactor);
#endif
    xsm->setBitOrder(outputRight, inputRight); // SWD is LSB, shift right both
    xsm->setPinDir(pin_swd, true);
    xsm->setPinDir(pin_clk, true);
    xsm->setPinDir(pin_direction, true);
    xsm->uploadCode(prgSizeInHalfWord, prg, wrapTarget, wrap);
    xsm->configure(pinConfig);
    xsm->configureSideSet(pin_direction, 2, 2, true);
    xsm->execute();
    lnPinModePIO(pin_swd, LN_SWD_PIO_ENGINE, true);
    lnPinModePIO(pin_clk, LN_SWD_PIO_ENGINE);
    lnPinModePIO(pin_direction, LN_SWD_PIO_ENGINE);
}


/**
 * @brief Reset GPIO to default state by cycling through pin modes.
 */
extern "C" void bmp_gpio_reset()
{
    bmp_pin_mode old_mode = currentPioMode;
    bmp_gpio_pinmode(BMP_PINMODE_SWD);
    bmp_gpio_pinmode(old_mode);
}
/**
 * @brief Switch GPIO/PIO mode (SWD, RVSWD, or bit-bang GPIO).
 * @param pioMode Desired pin mode.
 */
void bmp_gpio_pinmode(bmp_pin_mode pioMode)
{
    if (0 && pioMode == currentPioMode)
    {
        Logger(">>GPIO:Reusing previous PIO mode (%d)\n", currentPioMode);
        return;
    }
    currentPioMode = pioMode;
    lnPin pin_swd = _mapping[TSWDIO_PIN];
    lnPin pin_clk = _mapping[TSWDCK_PIN];
    switch (pioMode) // SWD, so PIO mode
    {
    case BMP_PINMODE_SWD:
        Logger(">>GPIO:Switching to PIO-SWD mode\n");
        setupPIO(sizeof(swd_program_instructions) / 2, swd_program_instructions, true, true, swd_wrap_target, swd_wrap,
                 SWD_PIO_RATE_FACTOR);
        break;
    case BMP_PINMODE_RVSWD:
        Logger(">>GPIO:Switching to PIO-RVSWD mode\n");
        setupPIO(sizeof(rvswd_program_instructions) / 2, rvswd_program_instructions, false, false, rvswd_wrap_target,
                 rvswd_wrap, RVSWD_PIO_RATE_FACTOR);
        break;
    case BMP_PINMODE_RVSWD_RAW:
        Logger(">>GPIO:Switching to PIO-RVSWD-RAW mode\n");
        setupPIO(sizeof(rvswd_raw_program_instructions) / 2, rvswd_raw_program_instructions, false, false,
                 rvswd_raw_wrap_target, rvswd_raw_wrap, RVSWD_PIO_RATE_FACTOR);
        break;
    case BMP_PINMODE_SDI:
        Logger(">>GPIO:Switching to PIO-SDI mode\n");
        sdi_pinmode_enter();
        break;
    case BMP_PINMODE_GPIO:
        Logger(">>GPIO:Switching to bitbanging mode\n");
        lnDigitalWrite(pin_swd, 1);
        lnDigitalWrite(pin_clk, 1);
        lnPinMode(pin_swd, lnOUTPUT);
        lnPinMode(pin_clk, lnOUTPUT);
        break;
    default:
        xAssert(0);
        break;
    }
}
/**
 * @brief Set SWD wait-state count (no-op for PIO mode).
 * @param ws Ignored.
 */
extern "C" void bmp_set_wait_state_c(uint32_t ws)
{
    Logger("Unsupported call in PIO mode\n");
    return;
}
/**
 * @brief Get SWD wait-state count (always 0 for PIO mode).
 * @return 0.
 */
extern "C" uint32_t bmp_get_wait_state_c()
{
    Logger("Unsupported call in PIO mode\n");
    return 0;
}
/**
 * @brief Begin SWD session: release target reset (Hi-Z).
 */
extern "C" void bmp_io_begin_session()
{
    pReset->off(); // hi-z by default
}
/**
 * @brief End SWD session: release target reset (Hi-Z).
 */
extern "C" void bmp_io_end_session()
{
    pReset->off(); // hi-z by default
} // EOF
