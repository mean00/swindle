/**
 * @file bmp_rvTap_ln.cpp
 * @brief RISC-V DMI transport over esprit GPIO (LN targets)
 */

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
 * @file bmp_rvTap_ln.cpp
 * @brief RISC-V DMI debug transport over bit-banged GPIO (esprit).
 *
 * Implements the WCH DMI serial protocol (start/stop framing, parity).
 * Derived from Blackmagic Probe RISC-V tap, rewritten for esprit GPIO.
 */
#include "esprit.h"

extern "C"
{
#include "jep106.h"
#include "riscv_debug.h"
}

#ifndef __clang__
#pragma GCC optimize("Ofast")
#endif
#include "bmp_pinout.h"
#include "bmp_rvTap.h"
#include "bmp_swdio_ln.h"
#include "bmp_tap_ln.h"
#include "esprit.h"
#include "lnBMP_reset.h"
#include "lnbmp_parity.h"
//--
extern void bmp_gpio_init();

uint64_t rvswd_write_then_read(uint64_t tx_data, int tx_bits, int rx_bits)
{
    // Start bit: SWDIO falling edge while CLK high
    rSWDIO->dir_output();
    rSWDIO->set(0);

    // TX Data: shift out MSB first
    if (tx_bits > 32)
    {
        int hi_bits = tx_bits - 32;
        uint32_t tx_hi = (uint32_t)(tx_data >> 32);
        uint32_t hi_mask = 1UL << (hi_bits - 1);
        for (int i = 0; i < hi_bits; i++)
        {
            rSWCLK->clockOff();
            rSWDIO->set(tx_hi & hi_mask ? 1 : 0);
            rSWCLK->clockOn();
            hi_mask >>= 1;
        }
        tx_bits = 32;
    }

    if (tx_bits > 0)
    {
        uint32_t tx_lo = (uint32_t)tx_data;
        uint32_t lo_mask = 1UL << (tx_bits - 1);
        for (int i = 0; i < tx_bits; i++)
        {
            rSWCLK->clockOff();
            rSWDIO->set(tx_lo & lo_mask ? 1 : 0);
            rSWCLK->clockOn();
            lo_mask >>= 1;
        }
    }

    // RX Data: shift in MSB first
    uint64_t rx_data = 0;
    if (rx_bits > 0)
    {
        rSWDIO->dir_input();
        uint32_t rx_hi = 0;
        uint32_t rx_lo = 0;
        int hi_bits = 0;

        if (rx_bits > 32)
        {
            hi_bits = rx_bits - 32;
            for (int i = 0; i < hi_bits; i++)
            {
                rSWCLK->clockOff();
                rSWCLK->clockOn();
                rx_hi = (rx_hi << 1) | (rSWDIO->read() ? 1 : 0);
            }
            rx_bits = 32;
        }

        for (int i = 0; i < rx_bits; i++)
        {
            rSWCLK->clockOff();
            rSWCLK->clockOn();
            rx_lo = (rx_lo << 1) | (rSWDIO->read() ? 1 : 0);
        }

        if (hi_bits > 0)
        {
            rx_data = ((uint64_t)rx_hi << 32) | rx_lo;
        }
        else
        {
            rx_data = rx_lo;
        }
    }

    // Stop bit: SWDIO rising edge while CLK high
    rSWCLK->clockOff();
    rSWDIO->dir_output();
    rSWDIO->set(0);
    rSWCLK->clockOn();
    rSWDIO->set(1);

    return rx_data;
}

/**
 * @brief Reset the RISC-V debug module via DMI line reset.
 *
 * Clock out 100 ones, then emit a stop bit and wait 10 ms.
 * @return true (always succeeds).
 */
bool rv_dm_reset()
{
    // toggle the clock 100 times
    rSWDIO->dir_output();
    rSWDIO->set(1);
    for (int i = 0; i < 100; i++)
    {
        rSWCLK->clockOff();
        rSWCLK->clockOn();
    }
    rSWDIO->set(0); // going low high with CLK = high => stop bit
    rSWDIO->set(1);
    lnDelayMs(10);
    return true;
}
#include "rvswd_template.h"
// EOF
