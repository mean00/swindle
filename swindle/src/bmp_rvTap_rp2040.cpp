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
extern "C" void bmp_rv_dm_read_c()
{
}
extern "C" void bmp_rv_dm_reset_c()
{
}
extern "C" void bmp_rv_dm_write_c()
{
}
/**
 * @brief
 *
 * @param adr
 * @param status
 * @return true
 * @return false
 */
bool LN_FAST_CODE rv_start_frame(uint32_t adr, uint32_t *status, bool wr)
{
    return false;
}
/**
 * @brief read 4 status bit then send a stop bit
 *
 * @return uint32_t
 */
bool LN_FAST_CODE rv_end_frame(uint32_t *status)
{
    return false;
}

/**
 */
bool LN_FAST_CODE rv_dm_write(uint32_t adr, uint32_t val)
{
    return false;
}
/**
 * @brief
 *
 * @param adr
 * @param output
 * @return true
 * @return false
 */
bool LN_FAST_CODE rv_dm_read(uint32_t adr, uint32_t *output)
{
    return false;
}
/**
 * @brief
 *
 * @return true
 * @return false
 */
bool rv_dm_reset()
{
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
    return true;
}

bool rv_dm_probe(uint32_t *chip_id)
{
    return false;
}
extern "C" bool rvswd_scan()
{
    return false;
}

// EOF
