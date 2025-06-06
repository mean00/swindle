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
#include "lnBMP_pinout.h"
extern "C"
{
#include "adiv5.h"
#include "general.h"
#include "timing.h"
}

#include "lnBMP_pinout.h"
#include "lnBMP_tap.h"
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
#include "platform_support.h"

static bmp_pin_mode currentPioMode;
extern uint32_t swd_frequency;
rpPIO *swdpio = NULL;
rpPIO_SM *xsm = NULL;
/**
 */
static void rp2040_swd_pio_change_clock(uint32_t fq)
{
    xsm->stop();
    xsm->setSpeed(fq);
    xsm->execute();
}

/**
 *
 */
extern "C" void bmp_set_frequency_c(uint32_t fq)
{
    swd_frequency = fq;
    rp2040_swd_pio_change_clock(fq * 4);
}
/**
 *
 *
 */
void bmp_gpio_init_extra()
{
    swdpio = new rpPIO(LN_SWD_PIO_ENGINE);
    xsm = swdpio->getSm(0);
    currentPioMode = BMP_PINMODE_NONE;
}
/**
 *
 *
 *
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
 *
 *
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
 *
 */
extern "C" void bmp_set_wait_state_c(uint32_t ws)
{
    Logger("Unsupported call in PIO mode\n");
    return;
}
/**
 *
 */
extern "C" uint32_t bmp_get_wait_state_c()
{
    Logger("Unsupported call in PIO mode\n");
    return 0;
}
// EOF
