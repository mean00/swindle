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
#include "lnBMP_tap.h"
#include "ln_rp_pio.h"
// clang-format on
#include "bmp_pinmode.h"
#include "lnRP2040_pio.h"
extern "C"
{
#include "hardware/structs/clocks.h"
    uint32_t clock_get_hz(enum clock_index clk_index);
#include "bmp_pio_rvswd.h"
#include "bmp_pio_swd.h"
#include "platform_support.h"
}
#include "lnbmp_parity.h"
extern void gmp_gpio_init_adc();
extern rpPIO *swdpio;
extern rpPIO_SM *xsm;

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
swd_proc_s swd_proc;
/**

*/
extern "C" void swdptap_init()
{
    bmp_gpio_pinmode(BMP_PINMODE_SWD);
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
