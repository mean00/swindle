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

#define SETCLOCK(p, v)      bmp_pin_set(p, v)
#define SETPIN(p, v)        bmp_pin_set(p, v)
#define GETPIN(p)           bmp_pin_get(p)
#define SET_OUTPUT(p)       bmp_pin_direction(p,1)
#define SET_INPUT(p)        bmp_pin_direction(p,0)
extern "C"
{
    void bmp_pin_set(uint8_t pin, uint8_t state);
    bool bmp_pin_get(uint8_t pin);
    void bmp_pin_direction(uint8_t pint, uint8_t is_output);
}
#endif

#define WRITE_BIT(x)    {SETCLOCK(pSWCLK, 0);   WAIT();      SETPIN(pSWDIO, !!(x));    WAIT();     SETCLOCK(pSWCLK, 1);}


bool parityOdd (uint32_t in)
{
    uint32_t  mid = in ^ (in >> 1);
    mid = mid ^ (mid >> 2);
    mid = mid ^ (mid >> 4);
    mid = mid ^ (mid >> 8);
    mid = mid ^ (mid >> 16);
    return mid & 1;
}
bool parityEven(uint32_t in)
{
    return !(parityOdd(in));
}

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
    WAIT();   WAIT();
    // Send start
    SETPIN(pSWDIO, 0);
    WAIT();
    //SETPIN(pSWCLK, 0); // start bit...
    WAIT();
   // SETCLOCK(pSWCLK, 1);
   // SETCLOCK(pSWCLK, 0);
    WAIT();
    int mask = 0x40;
    int value = host;

    // send address
    while (mask)
    {
        WRITE_BIT( (value & mask)  );
        mask >>= 1;
    }
    // Read/Write bit
    WRITE_BIT( write );

    // Parity: address  ^op
    int currentParity = (parityOdd(host)^ write);    
    WRITE_BIT( currentParity );

    // 5 bits@0
    WRITE_BIT( 0 );
    WRITE_BIT( 0 );
    WRITE_BIT( 0 );
    WRITE_BIT( 0 );
    WRITE_BIT( 0 );

    if (write)
    {
        uint32_t mask = 1UL << 31;
        uint32_t value = data;
        while (mask)
        {
            WRITE_BIT( (value & mask) );
            mask >>= 1;
        }
        // Parity target
        uint32_t parity=0; // TODO
        WRITE_BIT( parity );

        // 5 bits@0
        WRITE_BIT( 0 );
        WRITE_BIT( 0 );
        WRITE_BIT( 0 );
        WRITE_BIT( 1 );
        WRITE_BIT( 1 );

    }
    else // READ
    {        
        data = 0;
        for(int i=0;i<32;i++)
        {
            SETCLOCK(pSWCLK, 0);
            if(!i)
            {
                SET_INPUT(pSWDIO);
            }
            data = (data << 1) + GETPIN(pSWDIO);
            SETCLOCK(pSWCLK, 1);
        }
        uint32_t er = 0;
        for(int i=0;i<6;i++)
        {
            SETCLOCK(pSWCLK, 0);
            er = (er << 1) + GETPIN(pSWDIO);
            SETCLOCK(pSWCLK, 1);
        }
        SET_OUTPUT(pSWDIO);
        int parity= er >>5;
        er &= 0xf;
        printf("Out : 0x%x, parity =%x er=%x\n",data,parity,er);
    }
    
    // stop bit
    SETCLOCK(pSWCLK, 0);    
    SETPIN(pSWDIO, 0);
    SETCLOCK(pSWCLK, 1);    
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
    SETPIN(pSWDIO, 1);  // IDLE IS HIGH
    SET_OUTPUT(pSWDIO);
    SETCLOCK(pSWCLK, 1);
    SETPIN(pSWDIO, 1); 
    SET_OUTPUT(pSWCLK);
    SETPIN(pSWDIO, 1); 
    //SET_INPUT(pReset);
    //rv_wakeup();
}

extern "C" void rv_test(void)
{
    rvtap_init();
    uint32_t v;
    rv_write(0x10, 0x80000001);
    rv_read(0x11, &v);
#undef printf    
    printf("0x%x\n", v);
    rv_read(0x11, &v);
    printf("0x%x\n", v);
}

// EOF
