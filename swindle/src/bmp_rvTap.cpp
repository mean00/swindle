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
extern "C" uint32_t bmp_get_wait_state_c();

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

#warning this is duplicated from riscv_jtag_dtm
#define RV_DMI_NOOP 0U
#define RV_DMI_READ 1U
#define RV_DMI_WRITE 2U
#define RV_DMI_SUCCESS 0U
#define RV_DMI_FAILURE 2U
#define RV_DMI_TOO_SOON 3U

#define BMP_MIN_WS 1

bool rv_dm_reset();

extern SwdPin pSWDIO;
extern SwdWaitPin pSWCLK;
extern SwdReset pReset;

// data is sampled on transition clock low => clock high

#define PUT_BIT(x)                                                                                                     \
    pRVCLK.clockOff();                                                                                                 \
    pRVDIO.set(x);                                                                                                     \
    __asm__("nop");                                                                                                    \
    __asm__("nop");                                                                                                    \
    pRVCLK.clockOn();

#define READ_BIT(x)                                                                                                    \
    pRVCLK.clockOff();                                                                                                 \
    pRVCLK.clockOn();                                                                                                  \
    x = pRVDIO.read(); // read bit on rising edge
// 6
#define RV_WAIT()                                                                                                      \
    {                                                                                                                  \
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
 * @brief
 *
 * @param adr
 * @param status
 * @return true
 * @return false
 */
bool rv_start_frame(uint32_t adr, uint32_t *status, bool wr)
{
    // start bit
    pRVDIO.output();
    pRVDIO.off(); // io going low if CLK is high = start
    RV_WAIT();
    adr = (adr << 1) + wr;
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

    // last bit, switch to input if need be
    pRVCLK.clockOff();

    if (!wr)
    {
        pRVDIO.set(1);
        pRVDIO.input();
        pRVDIO.set(1);
    }
    else
    {
        pRVDIO.set(0);
    }
    pRVCLK.clockOn();

    return true;
}
/**
 * @brief read 4 status bit then send a stop bit
 *
 * @return uint32_t
 */
bool rv_end_frame(uint32_t *status)
{
    uint32_t out = 0;
    uint8_t bit;

    // now get the reply - 4 bits
    pRVCLK.clockOff();
    RV_WAIT();
    pRVDIO.input();

    READ_BIT(bit);
    out = (out << 1) + bit;
    READ_BIT(bit);
    out = (out << 1) + bit;
    READ_BIT(bit);
    out = (out << 1) + bit;
    READ_BIT(bit);
    out = (out << 1) + bit;

    pRVCLK.clockOff();

    pRVDIO.output();
    pRVDIO.set(0); // going high => stop bit
    pRVCLK.clockOn();
    pRVDIO.set(1); // going high => stop bit
    RV_WAIT();
    *status = out;
    if (out != 3 && out != 7)
    {
        Logger("Status error : 0x%x\n", out);
        return false;
    }
    return out;
}

/**
 */
bool rv_dm_write(uint32_t adr, uint32_t val)
{
    uint32_t status = 0;
    rv_start_frame(adr, &status, true);

    // Now data
    uint8_t *p = (uint8_t *)&val;
    int parity2 = parity8(p[0]) ^ parity8(p[1]) ^ parity8(p[2]) ^ parity8(p[3]);

    for (int i = 0; i < 32; i++)
    {
        PUT_BIT(val & 0x80000000UL);
        val <<= 1;
    }
    // data parity (twice)
    PUT_BIT(parity2);
    PUT_BIT(parity2);

    uint32_t st = 0;
    if (!rv_end_frame(&st))
    {
        Logger("Write failed Adr=0x%x Value=0x%x status=0x%x\n", adr, val, st);
        return false;
    }
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
    uint32_t status;
    rv_start_frame(adr, &status, false);

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

    uint32_t st = 0;
    if (!rv_end_frame(&st))
    {
        Logger("Read failed Adr=0x%x Value=0x%x status=0x%x\n", adr, value, st);
        return false;
    }
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
    // toggle the clock 100 times
    pRVDIO.output();
    pRVDIO.off(); //
    for (int i = 0; i < 100; i++)
    {
        PUT_BIT(1);
    }
    RV_WAIT();
    pRVDIO.set(0); // going low high with CLK = high => stop bit
    RV_WAIT();
    pRVDIO.set(1);
    RV_WAIT();
    lnDelayMs(10);
    return true;
}

#define WR(a, b) rv_dm_write(a, b)
#define RD(a, b) rv_dm_read(a, &out)

/**
 * @brief
 * 100 ws => 6 sec 0x5500 3kB
 * 20 ws => same
 * 10 ws => 8 sec
 * no fs => 12 sec
 * @return uint32_t
 */
bool rv_dm_start()
{
    int ws = bmp_get_wait_state_c();
    if (ws < BMP_MIN_WS)
        ws = BMP_MIN_WS;
    bmp_set_wait_state_c(ws);
    bmp_gpio_init();
    bmp_io_begin_session();

    rv_dm_reset();
    return true;
}

bool rv_dm_probe(uint32_t *chip_id)
{
    rv_dm_start();
    *chip_id = 0;

    uint32_t out = 0;
    //
    // init sequence, reverse eng from capture
    //----------------------------------------------
    WR(0x10, 0x80000001UL); // write DM CTROL =1
    lnDelayMs(10);
    WR(0x10, 0x80000001UL); // write DM CTRL = 0x800000001
    lnDelayMs(10);
    RD(0x11, 0x00030382UL); // read DM_STATUS
    lnDelayMs(10);
    RD(0x7f, 0x30700518UL); // read 0x7f
    *chip_id = out;         // 0x203xxxx 0x303xxxx 0x305...
    lnDelayMs(10);
    WR(0x05, 0x1ffff704UL);
    lnDelayMs(10);
    WR(0x17, 0x02200000UL);
    lnDelayMs(10);
    RD(0x04, 0x30700518UL);
    lnDelayMs(10);
    RD(0x05, 0x1ffff704UL);
    return (*chip_id) != 0xffffffffUL;
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
    int retries = 10;
    while (1)
    {
        if (!retries)
        {
            dmi->fault = RV_DMI_FAILURE;
            return false;
        }
        const bool result = rv_dm_read(address, value);
        if (result)
        {
            dmi->fault = RV_DMI_SUCCESS;
            return true;
        }
        retries--;
    }
#if 0    
    const bool result = rv_dm_read(address, value);
    if (result)
    {
        dmi->fault = RV_DMI_SUCCESS;
        return true;
    }
    dmi->fault = RV_DMI_FAILURE;
    return false;
#endif
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
    const bool result = rv_dm_write(address, value);
    if (result)
    {
        dmi->fault = RV_DMI_SUCCESS;
        return true;
    }
    dmi->fault = RV_DMI_FAILURE;
    return false;
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
    dmi->designer_code = JEP106_MANUFACTURER_WCH;
    dmi->version = RISCV_DEBUG_0_13; /* Assumption, unverified */
    dmi->address_width = 8U;
    dmi->read = ch32_riscv_dmi_read;
    dmi->write = ch32_riscv_dmi_write;

    riscv_dmi_init(dmi);

    return true;
}
extern "C"
{
    /**
     * @brief
     *
     * @param adr
     * @param value
     * @return true
     * @return false
     */
    extern "C" bool bmp_rv_dm_read_c(uint8_t adr, uint32_t *value)
    {
        bool r = rv_dm_read(adr, value);
        //   Logger("bmp_rv_dm_read_c : ad=0x%x value=0x%x status=%d\n",adr,*value,r);
        return r;
    }
    /**
     * @brief
     *
     * @param adr
     * @param value
     * @return true
     * @return false
     */
    extern "C" bool bmp_rv_dm_write_c(uint8_t adr, uint32_t value)
    {
        bool r = rv_dm_write(adr, value);
        //     Logger("bmp_rv_dm_write_c : ad=0x%x value=0x%x stat=%d\n",adr,value,r);
        return r;
    }
    /**
     * @brief
     *
     */
    extern "C" bool bmp_rv_dm_reset_c()
    {
        return rv_dm_reset();
    }
}

// EOF
