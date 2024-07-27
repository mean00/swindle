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

#include "lnBMP_pinout.h"
#include "lnBMP_swdio.h"
#include "ln_rp_pio.h"
// clang-format on
#include "lnRP2040_pio.h"
extern "C"
{
#include "hardware/structs/clocks.h"
    uint32_t clock_get_hz(enum clock_index clk_index);
#include "bmp_pio_swd.h"
}

extern void gmp_gpio_init_adc();
static uint32_t SwdRead(size_t ticks);
static bool SwdRead_parity(uint32_t *ret, size_t ticks);
static void SwdWrite(uint32_t MS, size_t ticks);
static void SwdWrite_parity(uint32_t MS, size_t ticks);

static rpPIO *swdpio;
rpPIO_SM *xsm;
uint32_t swd_delay_cnt = 4;
SwdReset pReset(TRESET_PIN);

/**
 *  write size bits over PIO
 */
static void zwrite(uint32_t size, uint32_t value)
{
    uint32_t zsize = ((size - 1) << 1) | 1;
    xsm->waitTxEmpty();
    xsm->write(1, &zsize);
    xsm->write(1, &value);
}
/**
 *  read size bits over PIO
 */
static uint32_t zread(uint32_t size)
{
    uint32_t zsize = ((size - 1) << 1) | 0;
    uint32_t value = 0;
    xsm->waitTxEmpty();
    xsm->write(1, &zsize);
    xsm->waitRxReady();
    xsm->read(1, &value);
    return value >> (32 - size);
}
/**
 */
static uint32_t getFqFromWs()
{
    uint32_t fq = clock_get_hz(clk_sys);
    fq = fq / (3 + swd_delay_cnt);
    return fq;
}
/**
 */
void rp2040_swd_pio_change_clock(uint32_t fq)
{
    xsm->stop();
    xsm->setSpeed(fq);
    xsm->execute();
}

/**
 * @brief
 *
 */
extern "C" void bmp_set_wait_state_c(uint32_t ws)
{
    swd_delay_cnt = ws;
    rp2040_swd_pio_change_clock(getFqFromWs());
}
/**
 * @brief
 *
 */
extern "C" uint32_t bmp_get_wait_state_c()
{
    return swd_delay_cnt;
}

/**

*/
void bmp_gpio_init()
{
    lnPin pin_swd = _mapping[TSWDIO_PIN];
    lnPin pin_clk = _mapping[TSWDCK_PIN];
    swdpio = new rpPIO(LN_SWD_PIO_ENGINE);
    xsm = swdpio->getSm(0);

    lnPinModePIO(pin_swd, LN_SWD_PIO_ENGINE, true);
    lnPinModePIO(pin_clk, LN_SWD_PIO_ENGINE);

    rpPIO_pinConfig pinConfig;
    pinConfig.sets.pinNb = 1;
    pinConfig.sets.startPin = pin_swd;
    pinConfig.outputs.pinNb = 1;
    pinConfig.outputs.startPin = pin_swd;
    pinConfig.inputs.pinNb = 1;
    pinConfig.inputs.startPin = pin_swd;

    xsm->setSpeed(getFqFromWs());
    xsm->setBitOrder(true, false);
    xsm->setPinDir(pin_swd, true);
    xsm->setPinDir(pin_clk, true);
    xsm->uploadCode(sizeof(swd_program_instructions) / 2, swd_program_instructions, swd_wrap_target, swd_wrap);
    xsm->configure(pinConfig);
    xsm->configureSideSet(pin_clk, 1, 2, true);
    xsm->execute();

    pReset.off(); // hi-z by default

    gmp_gpio_init_adc();
}
/**
 * @brief
 *
 */
void bmp_io_begin_session()
{
    pReset.off(); // hi-z by default
}
/**
 * @brief
 *
 */
void bmp_io_end_session()
{
    pReset.off(); // hi-z by default
}

/**
 */
static uint32_t SwdRead(size_t len)
{
    /*
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
    */
    return 0;
}
/**
 */
static bool SwdRead_parity(uint32_t *ret, size_t len)
{
    /*
    uint32_t res = 0;
    res = SwdRead(len);
    bool currentParity = __builtin_parity(res);
    bool parityBit = (pSWDIO.read() == 1);
    pSWCLK.clockOn();
    *ret = res;
    swdioSetAsOutput(true);
    return currentParity == parityBit; // should be equal
    */
    return 0;
}
/**

*/
static void SwdWrite(uint32_t MS, size_t ticks)
{
    /*
    // int cnt;
    swdioSetAsOutput(true);
    for (int i = 0; i < ticks; i++)
    {
        pSWCLK.clockOff();
        pSWDIO.set(MS & 1);
        pSWCLK.clockOn();
        MS >>= 1;
    }
    pSWCLK.clockOff();
    */
}
/**
 */
static void SwdWrite_parity(uint32_t MS, size_t ticks)
{

    /*
    bool parity = __builtin_parity(MS);
    SwdWrite(MS, ticks);
    pSWDIO.set(parity);
    pSWCLK.clockOn();
    pSWCLK.clockOff();
    */
}

/**
    properly invert SWDIO direction if needed
*/
static bool oldDrive = false;
void LN_FAST_CODE swdioSetAsOutput(bool output)
{
    if (output == oldDrive)
        return;
    oldDrive = output;

    switch ((int)output)
    {
    case false: // in
    {
        /*
        pSWDIO.input();
        pSWCLK.wait();
        pSWCLK.clockOn();
        */
        break;
    }
    break;
    case true: // out
    {
        /*
        pSWCLK.clockOff();
        pSWCLK.clockOn();
        pSWCLK.clockOff();
        pSWDIO.output();
        */
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
