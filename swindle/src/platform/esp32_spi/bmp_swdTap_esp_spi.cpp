/*
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
 */

/* This file implements the SW-DP interface. */

#include "esprit.h"
#include "bmp_pinout.h"
extern "C"
{
#include "adiv5.h"
#include "general.h"
#include "timing.h"
}
#include "bmp_pinmode.h"
#include "bmp_swdio_esp_spi.h"

#include "bmp_tap_esp_spi.h"
#include "lnbmp_parity.h"

#include "swd_tap_stubs.cpp"

//
#include "driver/spi_master.h"
#include "hal/gpio_types.h"

extern spi_device_handle_t swd_spi;
extern "C" void swdptap_init_stubs();
/*
 *
 */
void bmp_gpio_pinmode(bmp_pin_mode pioMode)
{
}
/*
 *
 */
extern "C" void swdptap_init()
{
    swdptap_init_stubs();
}
#define DIR_INPUT() rSWDIO->input()
#define DIR_OUTPUT() rSWDIO->output()
#define SWD_WAIT_PERIOD() swait()
#define SWINDLE_FAST_IO IRAM_ATTR

//____________________________________________
//____________________________________________
//____________________________________________
extern "C" bool SWINDLE_FAST_IO ln_adiv5_swd_write_no_check(const uint16_t addr, const uint32_t data)
{
    const uint8_t req = make_packet_request(ADIV5_LOW_WRITE, addr);
    uint8_t ack = 0;
    spi_transaction_ext_t t_ack = {
        .base =
            {
                .flags = SPI_TRANS_VARIABLE_ADDR | SPI_TRANS_VARIABLE_DUMMY,
                .addr = req,
                .rx_buffer = &ack,
            },
        .address_bits = 8,
        .dummy_bits = 1,
    };
    t_ack.base.rxlength = 3;
    spi_device_polling_transmit(swd_spi, (spi_transaction_t *)&t_ack);

    ack &= 0x07;
    if (ack != 1)
        return false; // FAULT or WAIT

    uint8_t p = lnOddParity(data);
    uint64_t tx_val = ((uint64_t)p << 32) | data;

    spi_transaction_ext_t t_data = {
        .base =
            {
                .flags = SPI_TRANS_VARIABLE_DUMMY,
                .tx_buffer = &tx_val,
            },
        .dummy_bits = 1, // Turnaround
    };
    t_data.base.length = 33 + 8;
    spi_device_polling_transmit(swd_spi, (spi_transaction_t *)&t_data);
    return true;
}
//
uint8_t rx_buf[6] = {0}; // To hold ACK (3) + Data (32) + Parity (1)
extern "C" SWINDLE_FAST_IO uint32_t ln_adiv5_swd_read_no_check(const uint16_t addr)
{
    const uint8_t req = make_packet_request(ADIV5_LOW_WRITE, addr);
    spi_transaction_ext_t t = {
        .base =
            {
                .flags = SPI_TRANS_VARIABLE_ADDR | SPI_TRANS_VARIABLE_DUMMY,
                .addr = req, // Send 8-bit request as "address" phase
                .rx_buffer = rx_buf,
            },
        .address_bits = 8,
        .dummy_bits = 1, // Turnaround cycle (1 clock)
    };
    t.base.rxlength = 36; // ACK(3) + Data(32) + Parity(1)
    spi_device_polling_transmit(swd_spi, (spi_transaction_t *)&t);

    if ((rx_buf[0] & 0x07) != SWD_ACK_OK)
    {
        Logger("Bad ack\n");
    }
    uint32_t data = (rx_buf[0] >> 3) | ((uint32_t)rx_buf[1] << 5) | ((uint32_t)rx_buf[2] << 13) |
                    ((uint32_t)rx_buf[3] << 21) | ((uint32_t)(rx_buf[4] & 0x0F) << 29);

    ln_raw_swd_write(8, 0);
    return data;
}
//
static bool SWINDLE_FAST_IO sendHeader(const uint8_t request, adiv5_debug_port_s *dp, const uint32_t cycles)
{
    platform_timeout_s timeout;
    platform_timeout_set(&timeout, 250U);
    uint8_t ack;
    do
    {
        uint8_t ack = 0;
        spi_transaction_ext_t t_ack = {
            .base =
                {
                    .flags = SPI_TRANS_VARIABLE_ADDR | SPI_TRANS_VARIABLE_DUMMY,
                    .addr = request,
                    .rx_buffer = &ack,
                },
            .address_bits = 8,
            .dummy_bits = 1,
        };
        t_ack.base.rxlength = 3;
        spi_device_polling_transmit(swd_spi, (spi_transaction_t *)&t_ack);
        ack &= 7;
        if (ack == SWD_ACK_OK)
            return true;
        if (ack == SWD_ACK_FAULT)
        {
            DEBUG_ERROR("SWD access resulted in fault, retrying\n");
            /* On fault, abort the request and repeat */
            /* Yes, this is self-recursive.. no, we can't think of a better option */
            //        swdptap_turnaround(SWDIO_STATUS_DRIVE);
            //        switch_to_output();
            adiv5_dp_write(dp, ADIV5_DP_ABORT,
                           ADIV5_DP_ABORT_ORUNERRCLR | ADIV5_DP_ABORT_WDERRCLR | ADIV5_DP_ABORT_STKERRCLR |
                               ADIV5_DP_ABORT_STKCMPCLR);
        }
        // something is wrong, retry
        ln_raw_swd_write(9, 0);
    } while ((ack == SWD_ACK_WAIT || ack == SWD_ACK_FAULT) && !platform_timeout_is_expired(&timeout));

    if (ack == SWD_ACK_WAIT)
    {
        DEBUG_ERROR("SWD access resulted in wait, aborting\n");
        dp->abort(dp, ADIV5_DP_ABORT_DAPABORT);
        dp->fault = ack;
        return false;
    }

    if (ack == SWD_ACK_FAULT)
    {
        DEBUG_ERROR("SWD access resulted in fault\n");
        dp->fault = ack;
        return false;
    }

    if (ack == SWD_ACK_NO_RESPONSE)
    {
        DEBUG_ERROR("SWD access resulted in no response\n");
        dp->fault = ack;
        return false;
    }

    if (ack != SWD_ACK_OK)
    {
        DEBUG_ERROR("SWD access has invalid ack %x\n", ack);
        raise_exception(EXCEPTION_ERROR, "SWD invalid ACK");
    }
    return true;
}
//
extern "C" uint32_t SWINDLE_FAST_IO ln_adiv5_swd_raw_access(adiv5_debug_port_s *dp, const uint8_t rnw,
                                                            const uint16_t addr, const uint32_t value)
{
    if ((addr & ADIV5_APnDP) && dp->fault)
        return 0;

    const uint8_t request = make_packet_request(rnw, addr);
    // read
    if (rnw)
    {
        if (!sendHeader(request, dp, 4))
        {
            return 0;
        }

        spi_transaction_ext_t t = {
            .base =
                {
                    .flags = SPI_TRANS_VARIABLE_ADDR | SPI_TRANS_VARIABLE_DUMMY,
                    .addr = 0, // Send 8-bit request as "address" phase
                    .rx_buffer = rx_buf,
                },
            .address_bits = 0,
            .dummy_bits = 1, // Turnaround cycle (1 clock)
        };
        t.base.rxlength = 33; // ACK(3) + Data(32) + Parity(1)

        spi_device_polling_transmit(swd_spi, (spi_transaction_t *)&t);
        uint32_t response = rx_buf[0] + (rx_buf[1] << 8) + (rx_buf[2] << 16) + (rx_buf[3] << 24);
        ln_raw_swd_write(8, 0);
        bool parity = lnOddParity(response);
        bool read_parity = rx_buf[4] & 1;
        if (read_parity != parity)
        {
            /* Give up on parity error */
            dp->fault = 1U;
            DEBUG_ERROR("SWD access resulted in parity error\n");
            raise_exception(EXCEPTION_ERROR, "SWD parity error");
        }
        return response;
    }
    // write
    if (!sendHeader(request, dp, 5))
    {
        return 0;
    }
    ln_raw_swd_write(1, 1);
    const bool parity = lnOddParity(value);
    ln_raw_swd_write(32, value);
    ln_raw_swd_write(9, parity);
    return 0;
}
//
static uint32_t tx1;
extern "C" void ln_raw_swd_write(uint32_t tick, uint32_t value)
{
    tx1 = value;
    spi_transaction_t t = {};
    t.length = tick;
    t.tx_buffer = &tx1;
    spi_device_polling_transmit(swd_spi, &t);
}
// EOF
//____________________________________________
