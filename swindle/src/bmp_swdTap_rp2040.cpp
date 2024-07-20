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
extern void gmp_gpio_init_adc();

uint32_t swd_delay_cnt = 4;
static void rp2040_swd_pio_init();
#include "lnBMP_swdio.h"
#if PC_HOSTED == 0
#include "lnBMP_pinout.h"
#endif

#include "ln_rp_pio.h"
// clang-format on
#include "lnRP2040_pio.h"


extern SwdPin pSWDIO;
extern SwdWaitPin pSWCLK; // automatically add delay after toggle

extern void swdioSetAsOutput(bool output);
#if 1
#define SWD_SPEED 200 * 1000UL
#else
#define SWD_SPEED 4 * 1000 * 1000UL
#endif
#define PICO_NO_HARDWARE 1
#define SWD_SM_TO_USE 0
#include "pio_swd.h"
static rpPIO *swdpio;
rpPIO_SM *xsm;



static uint32_t SwdRead(size_t ticks);
static bool SwdRead_parity(uint32_t *ret, size_t ticks);
static void SwdWrite(uint32_t MS, size_t ticks);
static void SwdWrite_parity(uint32_t MS, size_t ticks);

void swdioSetAsOutput(bool output);

SwdPin pSWDIO(TTRACE_PIN); // dummy
SwdWaitPin pSWCLK(TTRACE_PIN); // dummy
SwdReset pReset(TRESET_PIN);

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
extern "C" uint32_t bmp_get_wait_state_c()
{
    return swd_delay_cnt;
}

/**

*/
void bmp_gpio_init()
{
   
   
    pReset.hiZ(); // hi-z by default
    pReset.off(); // hi-z by default

    gmp_gpio_init_adc();
    rp2040_swd_pio_init();
}
/**
 * @brief
 *
 */
void bmp_io_begin_session()
{
  
    pReset.off();// hi-z by default
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
void rp2040_swd_pio_init()
{
    lnPin pin_swd = _mapping[TSWDIO_PIN];
    lnPin pin_clk = _mapping[TSWDCK_PIN];
    swdpio = new rpPIO(LN_SWD_PIO_ENGINE);
    xsm = swdpio->getSm(SWD_SM_TO_USE);

    lnPinMode(pin_swd, (lnGpioMode)(lnRP_PIO0_MODE + LN_SWD_PIO_ENGINE));
    lnPinMode(pin_clk, (lnGpioMode)(lnRP_PIO0_MODE + LN_SWD_PIO_ENGINE));

    rpPIO_pinConfig pinConfig;
    pinConfig.sets.pinNb = 1;
    pinConfig.sets.startPin = pin_swd;
    pinConfig.outputs.pinNb = 1;
    pinConfig.outputs.startPin = pin_swd;
    pinConfig.inputs.pinNb = 1;
    pinConfig.inputs.startPin = pin_swd;

    xsm->setSpeed(SWD_SPEED);
    xsm->setBitOrder(true, false);
    xsm->setPinDir(pin_swd, true);
    xsm->setPinDir(pin_clk, true);
    xsm->uploadCode(sizeof(swd_program_instructions) / 2, swd_program_instructions, swd_wrap_target, swd_wrap);
    xsm->configure(pinConfig);
    xsm->configureSideSet(pin_clk, 1, 2, true);
    xsm->execute();
    return;
}
/**
 */
static uint32_t SwdRead(size_t len)
{
   xAssert(0);
   return 0;
}
/**
 */
static bool SwdRead_parity(uint32_t *ret, size_t len)
{
    xAssert(0);
    return false;
}
/**

*/
static void SwdWrite(uint32_t MS, size_t ticks)
{
    xAssert(0);    
}
/**
 */
static void SwdWrite_parity(uint32_t MS, size_t ticks)
{
    xAssert(0);
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
