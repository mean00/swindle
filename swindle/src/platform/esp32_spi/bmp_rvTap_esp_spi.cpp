/**
 * @file      bmp_rvTap_esp_spi.cpp
 * @brief     RISC-V TAP (RVSWD) driver stub for ESP32 SPI-based debug probe.
 * @details   Derived from the Black Magic Debug project.
 *            All bit-bang operations are stubbed out because the ESP32
 *            uses the SPI peripheral for SWD signalling; RVSWD is not
 *            supported over the SPI path.
 *
 * Original license header:
 *
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
 *
 * This file implements the WCH DMI/Serial interface.
 *
 * CLK **HIGH**
 *         IO --___  START
 *         IO __---  STOP
 *
 * IO is sampled when clock goes ___---
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
#include "bmp_swdio_esp_spi.h"
#include "bmp_tap_esp_spi.h"
#include "esprit.h"
#include "lnBMP_reset.h"
#include "lnbmp_parity.h"
//--
extern void bmp_gpio_init();

/**
 * @brief Write @p n bits (stub — RVSWD not supported over SPI).
 */
static void rv_write_nbits(int n, uint32_t value)
{
    // value <<= (uint32_t)(32 - n);
    // const uint32_t mask = 0x80000000UL;
    // for (int i = 0; i < n; i++)
    //{
    // rSWCLK->clockOff();
    // rSWDIO->set(value & mask);
    // rSWCLK->clockOn();
    // value <<= 1;
    //}
}
/**
 * @brief Emit DMI start bit (stub — RVSWD not supported over SPI).
 */
static void rv_start_bit()
{
    // rSWDIO->dir_output();
    // rSWDIO->set(0);
}
/**
 * @brief Emit DMI stop bit (stub — RVSWD not supported over SPI).
 */
static void rv_stop_bit()
{
    // rSWCLK->clockOff();
    // rSWDIO->dir_output();
    // rSWDIO->set(0);
    // rSWCLK->clockOn();
    // rSWDIO->set(1);
}
/**
 * @brief Read @p n bits (stub — RVSWD not supported over SPI).
 * @return Always 0.
 */
static uint32_t rv_read_nbits(int n)
{
    // rSWDIO->dir_input();
    // uint32_t out = 0;
    // for (int i = 0; i < n; i++)
    //{
    // rSWCLK->clockOff();
    // rSWCLK->clockOn();
    // out = (out << 1) + rSWDIO->read(); // read bit on rising edge
    //}
    // return out;
    return 0;
}
/**
 * @brief Reset RISC-V DM (stub — always returns true).
 * @return true.
 */
bool rv_dm_reset()
{
    // toggle the clock 100 times
    //   rSWDIO->dir_output();
    // rSWDIO->set(1);
    // for (int i = 0; i < 5; i++) // 100 bits to 1
    //{
    // rv_write_nbits(20, 0xfffff);
    //}
    // rSWDIO->set(0); // going low high with CLK = high => stop bit
    // rSWDIO->set(1);
    // lnDelayMs(10);
    return true;
}
#include "rvswd_template.h"
// EOF
