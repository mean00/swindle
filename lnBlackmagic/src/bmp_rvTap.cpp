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
#include "rv.h"
}
extern uint32_t swd_delay_cnt;
#include "lnBMP_swdio.h"




extern SwdPin pSWDIO;
extern SwdWaitPin pSWCLK;
extern SwdPin pReset;

#define WAIT() {__asm__("nop");}

//  Start Adr(8bits)  Read(0)/Write(1) ParityHost(x) 5x0  32 bits ParityDevice(1) 5x0 Stop
// 10
// 5
// 32
// 7
// total = 22+32
static bool sendHeader(const uint8_t host, uint32_t &data, bool write)
{
        pSWDIO.on();        
        pSWCLK.clockOn();
        // Ok we are idle

        // Send start
        pSWDIO.off();
        WAIT();
        pSWCLK.off(); // start bit...
        WAIT();
        pSWCLK.clockOn();
        WAIT();
        int mask=0x80;
        int value=host;

        // send address
        while(mask)
        {
            pSWCLK.clockOff();
            pSWDIO.set(!!(value&mask));
            pSWCLK.clockOn();
            mask>>=1;
        }
        // Read/Write bit
        pSWCLK.clockOff();
        pSWDIO.set(write);
        pSWCLK.clockOn();
        // Parity: address  ^op
        int currentParity = (__builtin_popcount(host) + write) & 1;
        pSWCLK.clockOff();
        pSWDIO.set(currentParity);
        pSWCLK.clockOn();

        // 5 bits@0
        pSWCLK.clockOff(); pSWCLK.clockOn();
        pSWCLK.clockOff(); pSWCLK.clockOn();
        pSWCLK.clockOff(); pSWCLK.clockOn();
        pSWCLK.clockOff(); pSWCLK.clockOn();
        pSWCLK.clockOff(); pSWCLK.clockOn();

        if(write)
        {
            uint32_t mask=1<<31;
            int value=host;
            while(mask)
            {
                pSWCLK.clockOff();
                pSWDIO.set(!!(value&mask));
                pSWCLK.clockOn();
                mask>>=1;
            }
            // Parity target
            pSWCLK.clockOff();
            pSWDIO.set(0); // TODO
            pSWCLK.clockOn();

            // 5 bits@0
            pSWCLK.clockOff(); pSWCLK.clockOn();
            pSWCLK.clockOff(); pSWCLK.clockOn();
            pSWCLK.clockOff(); pSWCLK.clockOn();
            pSWCLK.clockOff(); pSWCLK.clockOn();
            pSWCLK.clockOff(); pSWCLK.clockOn();



        }else  // READ
        {
            pSWDIO.input();
            uint32_t mask=1<<31;
            data=0;
            while(mask)
            {
                pSWCLK.clockOff();
                data=(data<<1)+pSWDIO.read();
                pSWCLK.clockOn();
                mask>>=1;
            }
            // 5 bits@0
            pSWCLK.clockOff(); pSWCLK.clockOn();
            pSWCLK.clockOff(); pSWCLK.clockOn();
            pSWCLK.clockOff(); pSWCLK.clockOn();
            pSWCLK.clockOff(); pSWCLK.clockOn();
            pSWCLK.clockOff(); pSWCLK.clockOn();            
            pSWDIO.output();
        }

        // stop bit
        pSWCLK.clockOn();
        pSWDIO.set(0);
        pSWDIO.set(1);
        return true;
}
/**
*/
static bool rv_write(const uint8_t host, const uint32_t value)
{    
    uint32_t v=value;
    return sendHeader(host, v, true); 
}
/**
*/
static bool rv_read(const uint8_t host, uint32_t *value)
{
    return sendHeader(host, *value, false); 
}
/**
*/
static void rv_wakeup()
{
    pSWCLK.on();
    pSWDIO.set(1);
    for(int i=0;i<100;i++)
    {
        pSWCLK.clockOff();
        pSWCLK.clockOn();
    }
    pSWDIO.set(0);
    pSWDIO.set(1); // stop
}

//----------------
rv_proc_s rv_proc;
/**

*/
extern "C" void rvtap_init()
{
    rv_proc.write   = rv_write;
    rv_proc.read    = rv_read;
    rv_proc.wake_up = rv_wakeup;
}
// EOF
