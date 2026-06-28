/**
 * @file bmp_swdTap_fesp.cpp
 * @brief SWD bit-bang I/O over esprit GPIO (ESP32, fast-GPIO-driven)
 *
 * Original license from Black Magic Debug project:
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

#include "stdint.h"
//

#include "bmp_pinout.h"
#include "esprit.h"
extern "C"
{
#include "adiv5.h"
#include "general.h"
#include "timing.h"
}
#include "bmp_pinmode.h"

#define ESP_RUN_FAST __attribute__((optimize("O3"))) IRAM_ATTR

#include "bmp_swdio_fesp.h"

#include "bmp_tap_fesp.h"
#include "lnbmp_parity.h"

#include "swd_tap_stubs.cpp"

extern "C" void swdptap_init_stubs();
/**
 * @brief Set pin mode (stub — not needed for dedicated GPIO).
 */
void bmp_gpio_pinmode(bmp_pin_mode pioMode)
{
}
/**
 * @brief Initialise SWD TAP for ESP32 fast-GPIO.
 */
extern "C" void swdptap_init()
{
    swdptap_init_stubs();
}
/**
 * @brief Drive SWDIO high to release the target from reset.
 */
extern "C" void bmp_gpio_reset()
{
    rSWDIO->set(1);
    rSWDIO->output();
}
/*
 *
 */

/**
 * @brief Read @p size bits LSB-first from SWDIO (IRAM-safe, O3).
 * @param size Number of bits to read.
 * @return Sampled bits, LSB first.
 */
static uint32_t ESP_RUN_FAST zread(const size_t size)
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
/**
 * @brief Write @p size bits LSB-first on SWDIO (IRAM-safe, O3).
 * @param size Number of bits to write.
 * @param value Bits to write (only low @p size bits used).
 */
static void ESP_RUN_FAST zwrite(const uint32_t size, uint32_t value)
{
    xAssert(rSWDIO->dir());
    for (size_t i = 0; i < size; i++)
    {
        rSWDIO->set(value & 1);
        rSWCLK->invPulseClock();
        value >>= 1U;
    }
}

#define DIR_INPUT() rSWDIO->input()
#define DIR_OUTPUT() rSWDIO->output()
#define SWD_WAIT_PERIOD() swait()
#define SWINDLE_FAST_IO ESP_RUN_FAST

#include "swd_template.h"
//____________________________________________
