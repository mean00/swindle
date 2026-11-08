/**
 * @file swd_tap_stubs.cpp
 * @brief Shared SWD request-byte encoder
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
#include "platform_support.h"
}

extern "C" uint8_t make_packet_request(const uint8_t rnw, const uint16_t addr)
{
    uint8_t request = 0x81U;
    if (addr & ADIV5_APnDP)
        request ^= 0x22U;
    if (rnw)
        request ^= 0x24U;
    const uint8_t reg = addr & 0x0cU;
    request |= (reg << 1U) & 0x18U;
    if (reg == 4U || reg == 8U)
        request ^= 0x20U;
    return request;
}
// EOF
