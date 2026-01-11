/*
  lnBMP: Gpio driver for Rvswd
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


 This file implements the WCH DMI/Serial interface.

CLK **HIGH**
        IO --___  START
        IO __---  STOP

IO is sampled when clock goes ___---

 */
/**
 * This is similar to the non rp2040 except we switch to bit banging dynamically
 *
 */
#include "esprit.h"
#include "lnBMP_pinout.h"

extern "C"
{
#include "jep106.h"
#include "riscv_debug.h"
}

#ifndef __clang__
#pragma GCC optimize("Ofast")
#endif
#include "bmp_rvTap.h"
#include "esprit.h"
#include "lnBMP_pinout.h"
#include "lnBMP_swdio.h"
#include "lnBMP_tap.h"
#include "lnbmp_parity.h"
//--
extern void bmp_gpio_init();

/**
 *
 */
static void rv_write_nbits(int n, uint32_t value)
{
    xAssert(0);
}
/**
 * do a falling edge on SWDIO with CLK high (assumed) => start bit
 */
static void rv_start_bit()
{
    xAssert(0);
}
/**
 *
 * do a rising edge on SWDIO with CLK high (assumed) => stop bit
 */
static void rv_stop_bit()
{
    xAssert(0);
}
/**
 *
 */
static uint32_t rv_read_nbits(int n)
{
    xAssert(0);
    return false;
}
/**
 * @brief
 *
 * @return true
 * @return false
 */
bool rv_dm_reset()
{
    xAssert(0);
    return false;
}
#include "bmp_rvTap_common.h"
// EOF
