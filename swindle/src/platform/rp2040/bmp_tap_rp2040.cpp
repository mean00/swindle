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
#include "bmp_pio_sdi.h"
#include "bmp_pio_swd.h"
#include "lnBMP_reset.h"
#include "platform_support.h"

extern void gmp_gpio_init_adc();

static bmp_pin_mode currentPioMode;
uint32_t swd_frequency = 1000 * 1000; // 1Mhz by default
rpPIO *swdpio = NULL;
rpPIO_SM *xsm = NULL;
SwdReset *pReset;
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
    rp2040_swd_pio_change_clock(fq * 4);
}
/**
 * @brief One-time init of GPIO, PIO engine, ADC, and reset pin.
 */
void bmp_gpio_init_once()
{
    pReset = new SwdReset(TRESET_PIN); // automatically add delay after toggle
    pReset->setup();
    swdpio = new rpPIO(LN_SWD_PIO_ENGINE);
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
 */
static void setupPIO(int prgSizeInHalfWord, const uint16_t *prg, bool inputRight, bool outputRight, int wrapTarget,
                     int wrap)
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
    xsm->setSpeed(swd_frequency * 4);
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
 * @brief Upload and configure the single-wire SDI PIO program (WCH CH32V0xx).
 *
 * SDI is a one-wire pseudo open-drain protocol: only the shared SWDIO line is
 * used, there is no clock or direction side-set. The line idles high through
 * the pad pull-up and the PIO program pulls it low for each bit. When SDI is
 * selected after a SWD/RVSWD session the clock/direction pads are released
 * from PIO function so they do not float while the SWDIO pad stays on PIO.
 */
/* Requested SDI state-machine clock. rpPIO_SM::setSpeed() floors the divider
 * (intdiv = clk_sys / fq, see ln_rp_pio.cpp), so with clk_sys = 125 MHz
 * (LN_MCU_SPEED) this request for 16 MHz yields CLKDIV = 7 -> 17.857 MHz
 * (56 ns/cycle). That is what the sdi.pio bit schedule depends on: its 4-cycle
 * logic-1 low comes out at 224 ns, just under the fast-1x ceiling of 2T = 250 ns
 * (a /8 divider, 15.625 MHz, would stretch it to 256 ns and break the window).
 * Do not round this request up or change clk_sys without re-checking sdi.pio,
 * which carries the window table and the ~16-24 MHz valid band. */
#define SDI_PIO_FREQUENCY_HZ 16000000U
static void setupSDI()
{
    lnPin pin_sdi = _mapping[TSWDIO_PIN];
    lnPin pin_clk = _mapping[TSWDCK_PIN];
    lnPin pin_dir = _mapping[TDIRECTION_PIN];

    xsm->reset();
    lnDigitalWrite(pin_clk, 1);
    lnPinMode(pin_clk, lnOUTPUT);
    lnDigitalWrite(pin_dir, 1);
    lnPinMode(pin_dir, lnOUTPUT);

    lnPinModePIO(pin_sdi, LN_SWD_PIO_ENGINE, true); // PIO0 + pull-up (idle high)
    xsm->setSpeed(SDI_PIO_FREQUENCY_HZ);
    xsm->setBitOrder(false, false); // MSB-first: WCH SDI frames start at bit 31

    rpPIO_pinConfig pinConfig;
    pinConfig.sets.pinNb = 1;
    pinConfig.sets.startPin = pin_sdi;
    pinConfig.outputs.pinNb = 1;
    pinConfig.outputs.startPin = pin_sdi;
    pinConfig.inputs.pinNb = 1;
    pinConfig.inputs.startPin = pin_sdi;
    xsm->uploadCode(sizeof(sdi_program_instructions) / 2, sdi_program_instructions, sdi_wrap_target, sdi_wrap);
    xsm->configure(pinConfig);

    // Pseudo open-drain baseline: output level 0 while the direction floats.
    // pindirs=1 then drives a 0 (line low); pindirs=0 floats back high through
    // the pad pull-up.
    xsm->setPinsValue(0);
    xsm->setPinDir(pin_sdi, false);
    xsm->execute();
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
        setupPIO(sizeof(swd_program_instructions) / 2, swd_program_instructions, true, true, swd_wrap_target, swd_wrap);
        break;
    case BMP_PINMODE_RVSWD:
        Logger(">>GPIO:Switching to PIO-RVSWD mode\n");
        setupPIO(sizeof(rvswd_program_instructions) / 2, rvswd_program_instructions, false, false, rvswd_wrap_target,
                 rvswd_wrap);
        break;
    case BMP_PINMODE_RVSWD_RAW:
        Logger(">>GPIO:Switching to PIO-RVSWD-RAW mode\n");
        setupPIO(sizeof(rvswd_raw_program_instructions) / 2, rvswd_raw_program_instructions, false, false,
                 rvswd_raw_wrap_target, rvswd_raw_wrap);
        break;
    case BMP_PINMODE_SDI:
        Logger(">>GPIO:Switching to PIO-SDI mode\n");
        setupSDI();
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
