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
    // No critical section here: unlike SDI, where the cell timing *is* the encoding, an
    // RVSWD frame is clocked by the probe and the DM simply waits for the next clock, so
    // a gap between two bits (a USB interrupt, a preemption) is harmless. Masking would
    // only stall the USB ISR for a whole frame.
    //
    // Every data edge below is placed at the *start* of a clock phase, so the phase's own
    // wait() doubles as its setup time: the fix costs no clock time, only the ordering.
    //
    // Start bit: SWDIO falling edge while CLK high. The DM has to see that edge while CLK
    // is still high, so one wait() separates it from the first falling clock edge (once
    // per frame).
    rSWDIO->dir_output();
    rSWDIO->set(0);
    rSWCLK->wait();

    // TX Data: shift out MSB first.
    // Bit cell = low phase + high phase, one swait() each. The DM samples SWDIO on the
    // *rising* edge, so the data store is done at the *start* of the low phase: the low
    // phase's wait() is then the data setup time and the bit costs no extra clock time.
    // Storing the data at the end of the low phase (clockOff(); set(); clockOn();) leaves
    // only the GPIO store (~20 ns at 144 MHz, ~30 ns at 96 MHz) between the data edge and
    // the clock edge, and on the faster core the write frames lose that race.
    if (tx_bits > 32)
    {
        int hi_bits = tx_bits - 32;
        uint32_t tx_hi = (uint32_t)(tx_data >> 32);
        uint32_t hi_mask = 1UL << (hi_bits - 1);
        for (int i = 0; i < hi_bits; i++)
        {
            rSWCLK->off();                        // low phase starts
            rSWDIO->set(tx_hi & hi_mask ? 1 : 0); // data settles during it
            rSWCLK->wait();                       // low phase = data setup time
            rSWCLK->on();                         // rising edge, data already stable
            rSWCLK->wait();                       // high phase
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
            rSWCLK->off();                        // low phase starts
            rSWDIO->set(tx_lo & lo_mask ? 1 : 0); // data settles during it
            rSWCLK->wait();                       // low phase = data setup time
            rSWCLK->on();                         // rising edge, data already stable
            rSWCLK->wait();                       // high phase
            lo_mask >>= 1;
        }
    }

    // RX Data: shift in MSB first
    uint64_t rx_data = 0;
    if (rx_bits > 0)
    {
        // Release the wire and give the DM a wait state to start driving it: on a
        // single-wire bus the turnaround is not free, and the first response bit is
        // sampled one bit time later.
        rSWDIO->dir_input();
        rSWCLK->wait();
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

    // Stop bit: SWDIO rising edge while CLK high.
    // The line has to be at 0 *while* CLK is high, and only then rise, so the data edge
    // never coincides with the clock edge: drive 0, wait, CLK high, wait, drive 1.
    rSWCLK->clockOff();
    rSWDIO->dir_output();
    rSWDIO->set(0);
    rSWCLK->wait();  // 0 is stable before the clock edge
    rSWCLK->clockOn();
    rSWCLK->wait();  // CLK is high, line still 0
    rSWDIO->set(1);  // stop bit: data rises with CLK high

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
