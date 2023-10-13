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
#include "adiv5.h"
#include "general.h"
#include "timing.h"
}

uint32_t swd_delay_cnt = 4;

#include "lnBMP_swdio.h"
#if PC_HOSTED == 0
#include "../../../lnBlackmagic/private_include/lnBMP_pinout.h"
#include "lnADC.h"
#endif

static uint32_t SwdRead(size_t ticks);
static bool SwdRead_parity(uint32_t *ret, size_t ticks);
static void SwdWrite(uint32_t MS, size_t ticks);
static void SwdWrite_parity(uint32_t MS, size_t ticks);

static void swdioSetAsOutput(bool output);
static lnSimpleADC *adc = NULL;
static SwdPin pSWDIO(TSWDIO_PIN);
static SwdWaitPin pSWCLK(TSWDCK_PIN); // automatically add delay after toggle
static SwdReset pReset(TRESET_PIN);

/**
 * @brief 
 * 
 */
extern "C" void bmp_set_wait_state_c(uint32_t ws)
{
    swd_delay_cnt = ws;
}
/**
 * @brief 
 * 
 */
extern "C" uint32_t  bmp_get_wait_state_c()
{
    return swd_delay_cnt ;
}

/**

*/
void bmp_gpio_init()
{
    pSWDIO.on();
    pSWDIO.output();
    pSWCLK.clockOn();
    pSWCLK.output();
    pReset.off(); // hi-z by default
    lnPeripherals::enable(Peripherals::pADC0);
    lnPinMode(PIN_ADC_NRESET_DIV_BY_TWO, lnADC_MODE);
    adc = new lnSimpleADC(0, PIN_ADC_NRESET_DIV_BY_TWO);
    adc->setSmpt(LN_ADC_SMPT_239_5);
#if 0
    while(1)
    {
        __asm__("nop");
    }
#endif
}
/**
 */
static uint32_t SwdRead(size_t len)
{
    uint32_t index = 1;
    uint32_t ret = 0;
    int bit;

    swdioSetAsOutput(false);
    for (int i = 0; i < len; i++)
    {
        pSWCLK.clockOff();
        bit = pSWDIO.read();
        if (bit)
            ret |= index;
        pSWCLK.clockOn();
        index <<= 1;
    }
    pSWCLK.clockOff();
    return ret;
}
/**
 */
static bool SwdRead_parity(uint32_t *ret, size_t len)
{
    uint32_t res = 0;
    res = SwdRead(len);
    int currentParity = __builtin_popcount(res) & 1;
    int parityBit = pSWDIO.read();
    pSWCLK.clockOn();
    *ret = res;
    swdioSetAsOutput(true);
    return 1 & (currentParity ^ parityBit); // should be equal
}
/**

*/
static void SwdWrite(uint32_t MS, size_t ticks)
{
    int cnt;
    swdioSetAsOutput(true);
    for (int i = 0; i < ticks; i++)
    {
        pSWCLK.clockOff();
        pSWDIO.set(MS & 1);
        pSWCLK.clockOn();
        MS >>= 1;
    }
    pSWCLK.clockOff();
}
/**
 */
static void SwdWrite_parity(uint32_t MS, size_t ticks)
{
    int parity = __builtin_popcount(MS) & 1;
    SwdWrite(MS, ticks);
    pSWDIO.set(parity);
    pSWCLK.clockOn();
    pSWCLK.clockOff();
}

/**
    properly invert SWDIO direction if needed
*/
static bool oldDrive = false;
void swdioSetAsOutput(bool output)
{
    if (output == oldDrive)
        return;
    oldDrive = output;

    switch ((int)output)
    {
    case false: // in
    {
        pSWDIO.input();
        pSWCLK.clockOff();
        pSWCLK.clockOn();
        break;
    }
    break;
    case true: // out
    {
        pSWCLK.clockOff();
        pSWCLK.clockOn();
        pSWDIO.output();
        break;
    }
    default:
        break;
    }
}

/**
 */
extern "C" void platform_nrst_set_val(bool assert)
{
    if (assert) // force reset to low
    {
        pReset.on();
    }
    else // release reset
    {
        pReset.off();
    }
}
/**
 */
extern "C" bool platform_nrst_get_val(void)
{
    return pReset.state();
}

/*
 */

/*
 */
extern "C" float bmp_get_target_voltage_c()
{
    float vcc = 3300.; // lnBaseAdc::getVcc();
    if (vcc < 2600)
    {
        Logger("Invalid ADC Vref\n");
        return 0.0;
    }
    int sample = 0;
    for (int i = 0; i < 16; i++)
    {
        sample += adc->simpleRead();
    }
    sample /= 16;

    vcc = (float)sample * vcc * PIN_ADC_NRESET_MULTIPLIER; // need to multiply by PIN_ADC_NRESET_MULTIPLIER
    vcc = vcc / 4095000.;
    return vcc;
}

swd_proc_s swd_proc;
/**

*/
extern "C" void swdptap_init()
{
    swd_proc.seq_in = SwdRead;
    swd_proc.seq_in_parity = SwdRead_parity;
    swd_proc.seq_out = SwdWrite;
    swd_proc.seq_out_parity = SwdWrite_parity;
}

// EOF
