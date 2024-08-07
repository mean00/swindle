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
#include "lnbmp_parity.h"
extern void gmp_gpio_init_adc();

static uint32_t SwdRead(size_t ticks)
{
    xAssert(0);
    return 0;
}
static bool SwdRead_parity(uint32_t *ret, size_t ticks)
{
    xAssert(0);
    return 0;
}
static void SwdWrite(uint32_t MS, size_t ticks)
{
    xAssert(0);
}
static void SwdWrite_parity(uint32_t MS, size_t ticks)
{
    xAssert(0);
}

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
    fq = fq / (4 + swd_delay_cnt);
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

/**
    \fn ln_adiv5_swd_write_no_check
 */
extern "C" bool ln_adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data)
{
    uint8_t request = make_packet_request(ADIV5_LOW_WRITE, addr);
    zwrite(8, request);
    uint32_t ack = zread(3);
    bool parity = lnOddParity(data);
    zwrite(2, 0x03);
    zwrite(32, data);
    zwrite(1, parity);
    zwrite(8, 0);
    return ack != SWDP_ACK_OK;
}

/**
        \fn ln_adiv5_swd_read_no_check

 */
extern "C" uint32_t ln_adiv5_swd_read_no_check(const uint16_t addr)
{
    uint8_t request = make_packet_request(ADIV5_LOW_READ, addr);
    zwrite(8, request);
    uint32_t ack = zread(3);
    uint32_t data = zread(32);
    bool parity = zread(1);
    zwrite(8, 0);
    return ack == SWDP_ACK_OK ? data : 0;
}
/**
        \fn ln_adiv5_swd_raw_access

 */
extern "C" uint32_t ln_adiv5_swd_raw_access(adiv5_debug_port_s *dp, const uint8_t rnw, const uint16_t addr,
                                            const uint32_t value)
{
    //
    if ((addr & ADIV5_APnDP) && dp->fault)
        return 0;

    const uint8_t request = make_packet_request(rnw, addr);
    uint8_t ack = SWDP_ACK_WAIT;
    platform_timeout_s timeout;
    platform_timeout_set(&timeout, 250U);
    while (1)
    {

        zwrite(8, request);
        uint32_t ack = zread(3);
        bool expired = platform_timeout_is_expired(&timeout);
        if (ack == SWDP_ACK_OK)
        {
            goto done;
        }
        // if we get here, we are retrying
        // switch back to output
        zwrite(2, 0x03);
        switch (ack)
        {
        case SWDP_ACK_OK:
            xAssert(0);
            break;
        case SWDP_ACK_NO_RESPONSE:
            DEBUG_ERROR("SWD access resulted in no response\n");
            dp->fault = ack;
            return 0;

        case SWDP_ACK_WAIT:
            if (expired)
            {
                DEBUG_ERROR("SWD access resulted in wait, aborting\n");
                dp->abort(dp, ADIV5_DP_ABORT_DAPABORT);
                dp->fault = ack;
                return 0;
            }
            break;
        case SWDP_ACK_FAULT:
            if (expired)
            {
                DEBUG_ERROR("SWD access resulted in fault\n");
                dp->fault = ack;
                return 0;
            }
            DEBUG_ERROR("SWD access resulted in fault, retrying\n");
            /* On fault, abort the request and repeat */
            /* Yes, this is self-recursive.. no, we can't think of a better option */
            adiv5_dp_write(dp, ADIV5_DP_ABORT,
                           ADIV5_DP_ABORT_ORUNERRCLR | ADIV5_DP_ABORT_WDERRCLR | ADIV5_DP_ABORT_STKERRCLR |
                               ADIV5_DP_ABORT_STKCMPCLR);
            break;
        default:
            DEBUG_ERROR("SWD access has invalid ack %x\n", ack);
            raise_exception(EXCEPTION_ERROR, "SWD invalid ACK");
            break;
        }
    }
done:
    // We are out of read sequence, CLK is low
    if (rnw) // read ****************************************** HERE *************************
    {
        uint32_t index = 1;
        uint32_t response = zread(32);
        bool parityBit = zread(3) & 1;
        bool currentParity = lnOddParity(response);
        zwrite(8, 0);
        if (currentParity != parityBit)
        { /* Give up on parity error */
            dp->fault = 1U;
            DEBUG_ERROR("SWD access resulted in parity error\n");
            raise_exception(EXCEPTION_ERROR, "SWD parity error");
        }
        return response;
    }
    // write
    bool parity = lnOddParity(value);
    zwrite(2, 0x03);
    zwrite(32, value);
    zwrite(1, parity);
    zwrite(8, 0);
    return 0;
    //--
}

/**
 */
extern "C" void ln_raw_swd_write(uint32_t tick, uint32_t value)
{
    zwrite(tick, value);
}
// EOF
