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
#include "lnArduino.h"
#include "lnBMP_pinout.h"
extern "C"
{
#include "general.h"
#include "timing.h"
    // #include "rv.h"
}

#if 0
extern uint32_t swd_delay_cnt;
#include "lnBMP_swdio.h"

    extern SwdPin pSWDIO;
    extern SwdWaitPin pSWCLK;
    extern SwdPin pReset;

#else
#define pSWCLK 0
#define pSWDIO 1
#define pReset 0 // ??
#define WAIT()                                                                                                         \
    {                                                                                                                  \
        __asm__("nop");                                                                                                \
    }

#define SETCLOCK(p, v) bmp_pin_set(p, v)
#define SETPIN(p, v) bmp_pin_set(p, v)
#define GETPIN(x) bmp_pin_get(x)
#define SET_OUTPUT(x)                                                                                                  \
    {                                                                                                                  \
    }
#define SET_INPUT(x)                                                                                                   \
    {                                                                                                                  \
    }
extern "C"
{
    void bmp_pin_set(uint8_t pin, uint8_t state);
    bool bmp_pin_get(uint8_t pin);
}
#endif
//  Start Adr(8bits)  Read(0)/Write(1) ParityHost(x) 5x0  32 bits ParityDevice(1) 5x0 Stop
// 10
// 5
// 32
// 7
// total = 22+32
static bool send_rv_frame(const uint8_t host, uint32_t &data, bool write)
{
    SETPIN(pSWDIO, 1);
    SETCLOCK(pSWCLK, 1);
    // Ok we are idle

    // Send start
    SETPIN(pSWDIO, 0);
    WAIT();
    SETPIN(pSWCLK, 0); // start bit...
    WAIT();
    SETCLOCK(pSWCLK, 1);
    WAIT();
    int mask = 0x80;
    int value = host;

    // send address
    while (mask)
    {
        SETCLOCK(pSWCLK, 0);
        SETPIN(pSWDIO, !!(value & mask));
        SETCLOCK(pSWCLK, 1);
        mask >>= 1;
    }
    // Read/Write bit
    SETCLOCK(pSWCLK, 0);
    SETPIN(pSWDIO, write);
    SETCLOCK(pSWCLK, 1);
    // Parity: address  ^op
    int currentParity = (__builtin_popcount(host) + write) & 1;
    SETCLOCK(pSWCLK, 0);
    SETPIN(pSWDIO, currentParity);
    SETCLOCK(pSWCLK, 1);

    // 5 bits@0
    SETCLOCK(pSWCLK, 0);
    SETCLOCK(pSWCLK, 1);
    SETCLOCK(pSWCLK, 0);
    SETCLOCK(pSWCLK, 1);
    SETCLOCK(pSWCLK, 0);
    SETCLOCK(pSWCLK, 1);
    SETCLOCK(pSWCLK, 0);
    SETCLOCK(pSWCLK, 1);
    SETCLOCK(pSWCLK, 0);
    SETCLOCK(pSWCLK, 1);

    if (write)
    {
        uint32_t mask = 1 << 31;
        int value = host;
        while (mask)
        {
            SETCLOCK(pSWCLK, 0);
            SETPIN(pSWDIO, !!(value & mask));
            SETCLOCK(pSWCLK, 1);
            mask >>= 1;
        }
        // Parity target
        SETCLOCK(pSWCLK, 0);
        SETPIN(pSWDIO, 0); // TODO
        SETCLOCK(pSWCLK, 1);

        // 5 bits@0
        SETCLOCK(pSWCLK, 0);
        SETCLOCK(pSWCLK, 1);
        SETCLOCK(pSWCLK, 0);
        SETCLOCK(pSWCLK, 1);
        SETCLOCK(pSWCLK, 0);
        SETCLOCK(pSWCLK, 1);
        SETCLOCK(pSWCLK, 0);
        SETCLOCK(pSWCLK, 1);
        SETCLOCK(pSWCLK, 0);
        SETCLOCK(pSWCLK, 1);
    }
    else // READ
    {
        SET_INPUT(pSWDIO)
        uint32_t mask = 1 << 31;
        data = 0;
        while (mask)
        {
            SETCLOCK(pSWCLK, 0);
            data = (data << 1) + GETPIN(pSWDIO);
            SETCLOCK(pSWCLK, 1);
            mask >>= 1;
        }
        // 5 bits@0
        SETCLOCK(pSWCLK, 0);
        SETCLOCK(pSWCLK, 1);
        SETCLOCK(pSWCLK, 0);
        SETCLOCK(pSWCLK, 1);
        SETCLOCK(pSWCLK, 0);
        SETCLOCK(pSWCLK, 1);
        SETCLOCK(pSWCLK, 0);
        SETCLOCK(pSWCLK, 1);
        SETCLOCK(pSWCLK, 0);
        SETCLOCK(pSWCLK, 1);
        SET_OUTPUT(pSWDIO)
    }

    // stop bit
    SETCLOCK(pSWCLK, 1);
    SETPIN(pSWDIO, 0);
    SETPIN(pSWDIO, 1);
    return true;
}
/**
 */
static bool rv_write(const uint8_t host, const uint32_t value)
{
    uint32_t v = value;
    return send_rv_frame(host, v, true);
}
/**
 */
static bool rv_read(const uint8_t host, uint32_t *value)
{
    return send_rv_frame(host, *value, false);
}
/**
 */
static void rv_wakeup()
{
    SETPIN(pSWCLK, 1);
    SETPIN(pSWDIO, 1);
    for (int i = 0; i < 100; i++)
    {
        SETCLOCK(pSWCLK, 0);
        SETCLOCK(pSWCLK, 1);
    }
    SETPIN(pSWDIO, 0);
    SETPIN(pSWDIO, 1); // stop
}

//----------------
//rv_proc_s rv_proc;
/**

*/
extern "C" void rvtap_init()
{
    #if 0
    rv_proc.write = rv_write;
    rv_proc.read = rv_read;
    rv_proc.wake_up = rv_wakeup;
    #endif
    SETPIN(pSWDIO, 1);
    SET_OUTPUT(pSWDIO)
    SETCLOCK(pSWCLK, 1);
    SET_OUTPUT(pSWCLK);
    SET_INPUT(pReset);
    //rv_wakeup();
}

extern "C" void rv_test(void)
{
    rvtap_init();
    rv_write(0x10, 0x12345678);
}

// EOF
