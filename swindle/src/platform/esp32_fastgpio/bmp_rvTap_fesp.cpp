/**
 * @file bmp_rvTap_fesp.cpp
 * @brief ESP32 RISC-V DMI transport (fast GPIO inline)
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
 * This is similar to the non rp2040 except we switch to bit banging dynamically
 *
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
#include "bmp_swdio_fesp.h"
#include "bmp_tap_fesp.h"
#include "esprit.h"
#include "lnBMP_reset.h"
#include "lnbmp_parity.h"
//--
extern void bmp_gpio_init();

/**
 * @brief Write @p n bits MSB-first on SWDIO.
 * @param n     Number of bits (1..32).
 * @param value Bits left-aligned in 32-bit word.
 */
static void rv_write_nbits(int n, uint32_t value)
{
    value <<= (uint32_t)(32 - n);
    const uint32_t mask = 0x80000000UL;
    for (int i = 0; i < n; i++)
    {
        rSWCLK->clockOff();
        rSWDIO->set(value & mask);
        rSWCLK->clockOn();
        value <<= 1;
    }
}
/**
 * @brief Emit DMI start bit (SWDIO falling edge, CLK high).
 */
static void rv_start_bit()
{
    rSWDIO->dir_output();
    rSWDIO->set(0);
}
/**
 * @brief Emit DMI stop bit (SWDIO rising edge, CLK high).
 */
static void rv_stop_bit()
{
    rSWCLK->clockOff();
    rSWDIO->dir_output();
    rSWDIO->set(0);
    rSWCLK->clockOn();
    rSWDIO->set(1);
}
/**
 * @brief Read @p n bits MSB-first from SWDIO.
 * @param n Number of bits.
 * @return Sampled bits, MSB-aligned.
 */
static uint32_t rv_read_nbits(int n)
{
    rSWDIO->dir_input();
    uint32_t out = 0;
    for (int i = 0; i < n; i++)
    {
        rSWCLK->clockOff();
        rSWCLK->clockOn();
        out = (out << 1) + rSWDIO->read(); // read bit on rising edge
    }
    return out;
}
/**
 * @brief Reset RISC-V DM via DMI line reset.
 * @return true (always succeeds).
 */
bool rv_dm_reset()
{
    // toggle the clock 100 times
    rSWDIO->dir_output();
    rSWDIO->set(1);
    for (int i = 0; i < 5; i++) // 100 bits to 1
    {
        rv_write_nbits(20, 0xfffff);
    }
    rSWDIO->set(0); // going low high with CLK = high => stop bit
    rSWDIO->set(1);
    lnDelayMs(10);
    return true;
}
/**
 * @brief Write @p tx_bits bits of @p tx_data (MSB first), then read @p rx_bits
 *        bits (MSB first), with DMI start/stop framing.
 *
 * Used by rvswd_template.h (rv_dm_write/rv_dm_read). Ported from the generic
 * `ln` platform implementation to the ESP32 fast (dedicated) GPIO primitives.
 */
uint64_t rvswd_write_then_read(uint64_t tx_data, int tx_bits, int rx_bits)
{
    // Start bit: SWDIO falling edge while CLK high
    rSWDIO->dir_output();
    rSWDIO->set(0);

    // TX Data: shift out MSB first
    if (tx_bits > 0)
    {
        uint64_t v = tx_data;
        if (tx_bits < 64)
        {
            v <<= (uint64_t)(64 - tx_bits);
        }
        for (int i = 0; i < tx_bits; i++)
        {
            rSWCLK->clockOff();
            rSWDIO->set((uint32_t)((v >> 63) & 1ULL));
            rSWCLK->clockOn();
            v <<= 1;
        }
    }

    // RX Data: shift in MSB first
    uint64_t rx_data = 0;
    if (rx_bits > 0)
    {
        rSWDIO->dir_input();
        for (int i = 0; i < rx_bits; i++)
        {
            rSWCLK->clockOff();
            rSWCLK->clockOn();
            rx_data = (rx_data << 1) | (rSWDIO->read() ? 1ULL : 0ULL);
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
// Cooperative-yield hook: keeps the Task WDT fed (and the core responsive)
// during long DMI bit-bang bursts. See bmp_swd_yield in bmp_tap_fesp.cpp.
// Default (all other platforms) is a no-op.
#define SWD_TX_POLL() bmp_swd_yield()

#include "rvswd_template.h"

#undef SWD_TX_POLL
// EOF
