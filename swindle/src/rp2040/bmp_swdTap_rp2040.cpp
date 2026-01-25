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


  There is a one clock difference between the generic template and the PIO driven
  Not really sure where it's coming from



 */
#include "esprit.h"
extern "C"
{
#include "adiv5.h"
#include "general.h"
#include "timing.h"
}

#include "bmp_pinout.h"
#include "ln_rp_pio.h"
// clang-format on
#include "bmp_pinmode.h"
#include "lnRP2040_pio.h"
extern "C"
{
#include "bmp_pio_rvswd.h"
#include "bmp_pio_swd.h"
#include "platform_support.h"
}
#include "ln_rp_clocks.h"
#include "lnbmp_parity.h"
extern void gmp_gpio_init_adc();
extern rpPIO *swdpio;
extern rpPIO_SM *xsm;
extern "C" void swdptap_init_stubs();
/*
 *
 */
extern "C" void swdptap_init()
{
    bmp_gpio_pinmode(BMP_PINMODE_SWD);
    swdptap_init_stubs();
}

/**
 *  write size bits over PIO
 */
static void zwrite(uint32_t size, uint32_t value)
{
    uint32_t zsize = ((size - 1) << 1) | 1;
    xsm->write(1, &zsize);
    xsm->write(1, &value);
}
/**
 *  read size bits over PIO
 */
static uint32_t zread(uint32_t size)
{
    uint32_t zsize = ((size - 1) << 1) | 0;
    uint32_t value = 0;
    xsm->write(1, &zsize);
    xsm->waitRxReady();
    xsm->read(1, &value);
    return value >> (32 - size);
}

#define SWINDLE_FAST_IO
#define SWD_WAIT_PERIOD()                                                                                              \
    {                                                                                                                  \
    }
#define DIR_INPUT()                                                                                                    \
    {                                                                                                                  \
    }
#define DIR_OUTPUT()                                                                                                   \
    {                                                                                                                  \
    }

#include "../swd_template.h"
