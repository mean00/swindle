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

static uint32_t SwdRead(size_t len)
{
    xAssert(0);
    return 0;
}
static bool SwdRead_parity(uint32_t *ret, size_t len)
{
    xAssert(0);
    return true;
}
static void SwdWrite(uint32_t MS, size_t ticks)
{
    xAssert(0);
}
static void SwdWrite_parity(uint32_t MS, size_t ticks)
{
    xAssert(0);
}
/**

*/
swd_proc_s swd_proc;
extern "C" void swdptap_init_stubs()
{
    swd_proc.seq_in = SwdRead;
    swd_proc.seq_in_parity = SwdRead_parity;
    swd_proc.seq_out = SwdWrite;
    swd_proc.seq_out_parity = SwdWrite_parity;
}
// EOF
