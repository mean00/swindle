/*
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
 */

/* This file implements the SW-DP interface. */

#include "esprit.h"
#include "bmp_pinout.h"
extern "C"
{
#include "adiv5.h"
#include "general.h"
#include "timing.h"
}
#include "bmp_pinmode.h"
#include "bmp_swdio_esp.h"

#include "bmp_tap_esp.h"
#include "lnbmp_parity.h"

#include "../swd_tap_stubs.cpp"

extern "C" void swdptap_init_stubs();
/*
 *
 */
void bmp_gpio_pinmode(bmp_pin_mode pioMode)
{
}
/*
 *
 */
extern "C" void swdptap_init()
{
    swdptap_init_stubs();
}
/*
 *
 */

static uint32_t zread(const size_t size)
{
    xAssert(!rSWDIO->dir());
    uint32_t value = 0;
    uint32_t mask = 1;
    for (int i = 0; i < size; i++)
    {
        rSWCLK->wait();
        if (rSWDIO->read())
            value |= mask;
        rSWCLK->on();
        rSWCLK->invClockOff();
        mask <<= 1;
    }
    return value;
}
static void zwrite(const uint32_t size, uint32_t value)
{
    xAssert(rSWDIO->dir());
    for (size_t i = 0; i < size; i++)
    {
        rSWDIO->set(value & 1);
        rSWCLK->invPulseClock();
        value >>= 1U;
    }
}

//____________________________________________
#include "../swd_template.h"
//____________________________________________
