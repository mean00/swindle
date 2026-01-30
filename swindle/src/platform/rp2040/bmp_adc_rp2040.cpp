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
#include "bmp_pinout.h"
#include "lnGPIO.h"
extern "C"
{
#include "adiv5.h"
#include "general.h"
#include "timing.h"
}
#include "lnADC.h"
/**
 * @brief
 *
 */
static lnSimpleADC *adc = NULL;
void gmp_gpio_init_adc()
{
    if (!adc)
    {
        adc = new lnSimpleADC(0, PIN_ADC_NRESET_DIV_BY_TWO); // this pins is connected to NRST/2
    }
}

/*
 */
extern "C" float bmp_get_target_voltage_c()
{
    xAssert(adc);
    float vcc = (float)adc->simpleRead(16);
    vcc = vcc * 3.3;
    vcc /= 4095.;
    return vcc;
}
// EOF
