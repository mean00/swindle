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
#include "lnArduino.h"
#include "lnBMP_pinout.h"

extern "C"
{
#include "jep106.h"
#include "riscv_debug.h"
}

extern "C" void bmp_set_wait_state_c(uint32_t ws);
extern void bmp_io_begin_session();
extern void bmp_gpio_init();
extern uint32_t swd_delay_cnt;

#include "lnBMP_swdio.h"
#if PC_HOSTED == 0
#include "lnBMP_pinout.h"
#endif

#include "bmp_rvTap.h"

#define pRVDIO pSWDIO
#define pRVCLK pSWCLK
#define Rvswd_delay_cnt swd_delay_cnt

static bool rv_dm_reset();

extern SwdPin pSWDIO;
extern SwdWaitPin pSWCLK;
extern SwdReset pReset;

// data is sampled on transition clock low => clock high

#define PUT_BIT(x)                                                                                                     \
    pRVCLK.off();                                                                                                      \
    pRVDIO.set(x);                                                                                                     \
    pRVCLK.on();

#define READ_BIT(x)                                                                                                    \
    pRVCLK.off();                                                                                                      \
    pRVCLK.on();                                                                                                       \
    x = pRVDIO.read(); // read bit on rising edge

#define RV_WAIT()                                                                                                      \
    {                                                                                                                  \
        __asm__("nop");                                                                                                \
        __asm__("nop");                                                                                                \
        __asm__("nop");                                                                                                \
        __asm__("nop");                                                                                                \
    }

static const uint8_t par_table[16] = {0, 1, 1, 0, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 1, 0};
/**
 * @brief
 *
 * @param x
 * @return int
 */
static inline int parity8(uint8_t x)
{
    return (par_table[x >> 4] ^ par_table[x & 0xf]);
}

/**
 */
bool rv_dm_write(uint32_t adr, uint32_t val)
{
    // start bit
    pRVDIO.output();

    pRVDIO.off(); // io going low if CLK is high = start
    RV_WAIT();
    // pRVCLK.clockOff();

    adr <<= 1;
    adr |= 1; // write
    int parity = parity8(adr);
    for (int i = 0; i < 8; i++)
    {
        PUT_BIT(adr & 0x80);
        adr <<= 1;
    }
    // send parity twice
    PUT_BIT(parity);
    PUT_BIT(parity);

    // dont know if this is from host or target
    PUT_BIT(0);
    PUT_BIT(0);
    PUT_BIT(0);
    PUT_BIT(0);

    // Now data
    uint8_t *p = (uint8_t *)&val;
    int parity2 = parity8(p[0]) ^ parity8(p[1]) ^ parity8(p[2]) ^ parity8(p[3]);

    for (int i = 0; i < 32; i++)
    {
        PUT_BIT(val & 0x80000000UL);
        val <<= 1;
    }
    // data partity (twice)
    PUT_BIT(parity2);
    PUT_BIT(parity2);

    // now get the reply - 4 bits
    pRVCLK.off();
    pRVDIO.input();

    uint8_t bit;
    READ_BIT(bit);
    READ_BIT(bit);
    READ_BIT(bit);
    READ_BIT(bit);

    pRVCLK.off();
    RV_WAIT();
    pRVDIO.output();
    pRVDIO.set(0); // going high => stop bit
    pRVCLK.on();
    RV_WAIT();
    pRVDIO.set(1); // going high => stop bit
    RV_WAIT();

    return true;
}
/**
 * @brief
 *
 * @param adr
 * @param output
 * @return true
 * @return false
 */
bool rv_dm_read(uint32_t adr, uint32_t *output)
{
    // start bit
    pRVDIO.output();

    pRVDIO.off(); // io going low if CLK is high = start
    RV_WAIT();
    // pRVCLK.clockOff();

    adr <<= 1;
    adr |= 0; // read
    int parity = parity8(adr);
    for (int i = 0; i < 8; i++)
    {
        PUT_BIT(adr & 0x80);
        adr <<= 1;
    }
    // send parity twice
    PUT_BIT(parity);
    PUT_BIT(parity);

    // dont know if this is from host or target

    PUT_BIT(0);
    PUT_BIT(0);
    PUT_BIT(0);
    // PUT_BIT(0);
    pRVCLK.off();
    pRVDIO.set(0);
    pRVDIO.input();
    pRVCLK.on(); /**
                  */

    uint32_t value = 0, bit;
    for (int i = 0; i < 32; i++)
    {
        READ_BIT(bit);
        value <<= 1;
        value += bit;
    }
    *output = value;

    READ_BIT(bit); // read parity bit
    READ_BIT(bit); // read parity bit
    // status x4
    READ_BIT(bit); // read parity bit
    READ_BIT(bit); // read parity bit
    READ_BIT(bit); // read parity bit
    // Last status we let clk low
    pRVCLK.off();
    RV_WAIT();

    bit = pRVDIO.read();

    // clk is high, we can go low up = stop bit
    pRVDIO.set(0);
    pRVDIO.output();
    RV_WAIT();

    pRVCLK.on();
    RV_WAIT();

    pRVDIO.set(1);
    RV_WAIT();
    return true;
}
/**
 * @brief
 *
 * @return true
 * @return false
 */
bool rv_dm_reset()
{
    // toggle the clock 50 times
    pRVDIO.output();
    pRVDIO.off(); //
    for (int i = 0; i < 100; i++)
    {
        PUT_BIT(1);
        RV_WAIT();
    }
    RV_WAIT();
    pRVDIO.set(1); // going high => stop bit
    RV_WAIT();
    return true;
}

#define WR(a, b) rv_dm_write(a, b)
#define RD(a, b) rv_dm_read(a, &out)

/**
 * @brief
 *
 * @return uint32_t
 */
bool rv_dm_probe(uint32_t *chip_id)
{
    bmp_set_wait_state_c(100);
    bmp_gpio_init();
    bmp_io_begin_session();

    rv_dm_reset();
    *chip_id = 0;

    uint32_t out = 0;
    // init sequence
    WR(0x10, 0x80000001UL);
    lnDelayMs(10);
    WR(0x10, 0x80000001UL);
    lnDelayMs(10);
    RD(0x11, 0x00030382UL);
    lnDelayMs(10);
    RD(0x7f, 0x30700518UL);
    *chip_id = out; // 0x203xxxx 0x303xxxx 0x305...
    lnDelayMs(10);
    WR(0x05, 0x1ffff704UL);
    lnDelayMs(10);
    WR(0x17, 0x02200000UL);
    lnDelayMs(10);
    RD(0x04, 0x30700518UL);
    lnDelayMs(10);
    RD(0x05, 0x1ffff704UL);
    return true;
}
/**
 * @brief
 *
 * @param dmi
 * @param address
 * @param value
 * @return true
 * @return false
 */
static bool ch32_riscv_dmi_read(riscv_dmi_s *const dmi, const uint32_t address, uint32_t *const value)
{
    uint8_t status = 0;
    const bool result = rv_dm_read(address, value);

    /* Translate error 1 into RV_DMI_FAILURE per the spec, also write RV_DMI_FAILURE if the transfer failed */
    dmi->fault = !result || status == 1U ? RV_DMI_FAILURE : status;
    return dmi->fault == RV_DMI_SUCCESS;
}
/**
 * @brief
 *
 * @param dmi
 * @param address
 * @param value
 * @return true
 * @return false
 */
static bool ch32_riscv_dmi_write(riscv_dmi_s *const dmi, const uint32_t address, const uint32_t value)
{
    uint8_t status = 0;
    const bool result = rv_dm_write(address, value);

    /* Translate error 1 into RV_DMI_FAILURE per the spec, also write RV_DMI_FAILURE if the transfer failed */
    if (!result)
        dmi->fault = RV_DMI_FAILURE;
    else
        dmi->fault = RV_DMI_SUCCESS;
    return dmi->fault == RV_DMI_SUCCESS;
}

/**
 * @brief
 *
 */
extern "C" void target_list_free(void);
extern "C" bool rvswd_scan()
{
    uint32_t id = 0;
    target_list_free();
    if (!rv_dm_probe(&id))
    {
        return false;
    }
    Logger("WCH : found 0x%x device\n", id);
    riscv_dmi_s *dmi = new riscv_dmi_s;
    memset(dmi, 0, sizeof(*dmi));
    if (!dmi)
    { /* calloc failed: heap exhaustion */
        Logger("calloc: failed in %s\n", __func__);
        return false;
    }
    dmi->designer_code = NOT_JEP106_MANUFACTURER_WCH;
    dmi->version = RISCV_DEBUG_0_13; /* Assumption, unverified */
    dmi->address_width = 8U;
    dmi->read = ch32_riscv_dmi_read;
    dmi->write = ch32_riscv_dmi_write;

    riscv_dmi_init(dmi);

    return true;
}

// EOF