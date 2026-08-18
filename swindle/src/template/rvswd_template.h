/**
 * @file rvswd_template.h
 * @brief RISC-V DMI serial protocol template (included by platform files)
 */

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
/**
 * This is similar to the non rp2040 except we switch to bit banging dynamically
 *
 */
#include "esprit.h"
// #include "lnBMP_pinout.h"

extern "C"
{
#include "jep106.h"
#include "riscv_debug.h"
}
#ifndef __clang__
#pragma GCC optimize("Ofast")
#endif
#include "bmp_pinmode.h"
#include "bmp_rvTap.h"
#include "esprit.h"
// #include "lnBMP_swdio.h"
// #include "lnBMP_tap.h"
#include "lnbmp_parity.h"
//--
extern void bmp_gpio_init();

extern "C" void bmp_set_wait_state_c(uint32_t ws);
extern "C" uint32_t bmp_get_wait_state_c();

// #warning this is duplicated from riscv_jtag_dtm
#define RV_DMI_SUCCESS 0U
#define RV_DMI_FAILURE 2U

// ---------------------------------------------------------------
// Phase 0 instrumentation: global RVSWD (DMI) transaction counter.
// Incremented once per complete DMI request/response exchange.
// Definition lives in bmp_interface_c.cpp (single TU).
// ---------------------------------------------------------------
extern "C" volatile uint32_t ln_rv_tx_count;

// ---------------------------------------------------------------
// Cooperative-yield hook. Same contract as in swd_template.h:
// platforms that bit-bang for a long time (e.g. ESP32 with its Task
// Watchdog) override this to periodically yield the CPU. Invoked
// once per DMI transaction, between wire exchanges. Default: no-op.
// ---------------------------------------------------------------
#ifndef SWD_TX_POLL
#define SWD_TX_POLL() \
    do               \
    {                \
    } while (0)
#endif

#define BMP_MIN_WS 1

bool LN_FAST_CODE rv_dm_write(uint32_t adr, uint32_t val);
bool LN_FAST_CODE rv_dm_read(uint32_t adr, uint32_t *output);
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
    bmp_gpio_pinmode(BMP_PINMODE_RVSWD);
    int ws = bmp_get_wait_state_c();
    if (ws < BMP_MIN_WS)
        ws = BMP_MIN_WS;
    bmp_set_wait_state_c(ws);
    rv_dm_reset();
    return true;
}
/**
 */
extern "C" void rv_dm_start_c()
{
    rv_dm_start();
}

/**
 */
bool LN_FAST_CODE rv_dm_write(uint32_t adr, uint32_t val)
{
    ln_rv_tx_count++;
    SWD_TX_POLL();
    uint64_t tx = (adr << 1) + 1; // 1 = write
    int parity1 = lnOddParity(tx);
    tx = (tx << 2) | (parity1 ? 3 : 0);

    // Padding: 4 bits of 0
    tx = (tx << 4) | 0;

    // Data phase: 32 bits
    tx = (tx << 32) | val;

    // Data parity: 2 bits
    int parity2 = lnOddParity(val);
    tx = (tx << 2) | (parity2 ? 3 : 0);

    // TX total = 10 + 4 + 32 + 2 = 48 bits
    // RX total = 4 bits (status)
    uint64_t rx = rvswd_write_then_read(tx, 48, 4);

    if (rx != 3 && rx != 7)
    {
        // Quiet here: a non-OK status is usually "busy" (a system-bus access is still in
        // flight), which the DMI layer retries with a bounded budget (BMP 85). Log only
        // when the retries are exhausted.
        return false;
    }
    return true;
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
    ln_rv_tx_count++;
    SWD_TX_POLL();
    uint64_t tx = (adr << 1) + 0; // 0 = read
    int parity1 = lnOddParity(tx);
    tx = (tx << 2) | (parity1 ? 3 : 0);

    // Padding: 4 bits of 1 (0001)
    tx = (tx << 4) | 1;

    // TX total = 14 bits
    // RX total = 32 (data) + 2 (parity) + 4 (status) = 38 bits
    uint64_t rx = rvswd_write_then_read(tx, 14, 38);

    uint32_t status = rx & 0xF;
    *output = (rx >> 6) & 0xFFFFFFFF;

    if (status != 3 && status != 7)
    {
        // Quiet here: a non-OK status is usually "busy" (a system-bus access is still in
        // flight), which the DMI layer retries with a bounded budget (BMP 85). Log only
        // when the retries are exhausted.
        return false;
    }
    return true;
}
#define WR(a, b) rv_dm_write(a, b)
#define RD(a, b) rv_dm_read(a, &out)

/**
 *
 *
 */
static bool rv_dm_probe(uint32_t *chip_id)
{
#define DELAY() lnDelayMs(1)
    *chip_id = 0;

    uint32_t out = 0;
    //
    // init sequence, reverse eng from capture
    //----------------------------------------------
    WR(0x10, 0x80000001UL); // write DM CTROL =1
    DELAY();
    WR(0x10, 0x80000001UL); // write DM CTRL = 0x800000001
    DELAY();
    RD(0x11, 0x00030382UL); // read DM_STATUS
    DELAY();
    RD(0x7f, 0x30700518UL); // read 0x7f
    *chip_id = out;         // 0x203xxxx 0x303xxxx 0x305...
    DELAY();
    WR(0x05, 0x1ffff704UL);
    DELAY();
    WR(0x17, 0x02200000UL);
    DELAY();
    RD(0x04, 0x30700518UL);
    DELAY();
    RD(0x05, 0x1ffff704UL);
    return ((*chip_id) & 0x7fff) != 0x7fff;
}
/**
 * @brief
 *
 * @param dmi
 * @param address
 * @param value
 * @return true
 * @return false
 */
/* BMP 85: bounded busy-in-status retry budget for DMI accesses. A non-OK DMI response
 * usually means the DM is still busy with the previous system-bus access (or the DM
 * latched the access only recently, e.g. right after an ADDR0/DATA0 write). Retry a
 * bounded number of times, then report failure - the riscv32 sysbus code then falls
 * back to the proven CTRLSTATUS busy-poll. */
#define RVSWD_DMI_MAX_ATTEMPTS 4U

static bool ch32_riscv_dmi_read(riscv_dmi_s *const dmi, const uint32_t address, uint32_t *const value)
{
    for (uint32_t attempt = 0U; attempt < RVSWD_DMI_MAX_ATTEMPTS; ++attempt)
    {
        if (rv_dm_read(address, value))
        {
            dmi->fault = RV_DMI_SUCCESS;
            return true;
        }
    }
    Logger("DMI read failed Adr=0x%x after %u attempts\n", address, (unsigned)RVSWD_DMI_MAX_ATTEMPTS);
    dmi->fault = RV_DMI_FAILURE;
    return false;
}
/**
 * @brief
 *
 * @param dmi
 * @param address
 * @param value
 * @return true
 * @return false
 */
static bool ch32_riscv_dmi_write(riscv_dmi_s *const dmi, const uint32_t address, const uint32_t value)
{
    for (uint32_t attempt = 0U; attempt < RVSWD_DMI_MAX_ATTEMPTS; ++attempt)
    {
        if (rv_dm_write(address, value))
        {
            dmi->fault = RV_DMI_SUCCESS;
            return true;
        }
    }
    Logger("DMI write failed Adr=0x%x after %u attempts\n", address, (unsigned)RVSWD_DMI_MAX_ATTEMPTS);
    dmi->fault = RV_DMI_FAILURE;
    return false;
}
extern "C"
{
    /**
     * @brief
     *
     * @param adr
     * @param value
     * @return true
     * @return false
     */
    extern "C" bool bmp_rv_dm_read_c(uint8_t adr, uint32_t *value)
    {
        bool r = rv_dm_read(adr, value);
        //   Logger("bmp_rv_dm_read_c : ad=0x%x value=0x%x status=%d\n",adr,*value,r);
        return r;
    }
    /**
     * @brief
     *
     * @param adr
     * @param value
     * @return true
     * @return false
     */
    extern "C" bool bmp_rv_dm_write_c(uint8_t adr, uint32_t value)
    {
        bool r = rv_dm_write(adr, value);
        //     Logger("bmp_rv_dm_write_c : ad=0x%x value=0x%x stat=%d\n",adr,value,r);
        return r;
    }
    /**
     * @brief
     *
     */
    extern "C" bool bmp_rv_dm_reset_c()
    {
        return rv_dm_reset();
    }
}

/**
 * @brief
 *
 */
extern "C" void target_list_free(void);
extern "C" bool rvswd_scan()
{
    uint32_t id = 0;
    rv_dm_start();
    target_list_free();
    if (!rv_dm_probe(&id))
    {
        return false;
    }
    Logger("WCH : found 0x%x device\n", id);
    riscv_dmi_s *dmi = new riscv_dmi_s;
    if (!dmi)
    { /* calloc failed: heap exhaustion */
        Logger("calloc: failed in %s\n", __func__);
        return false;
    }
    memset(dmi, 0, sizeof(*dmi));
    dmi->designer_code = JEP106_MANUFACTURER_WCH;
    dmi->version = RISCV_DEBUG_0_13; /* Assumption, unverified */
    dmi->address_width = 8U;
    dmi->read = ch32_riscv_dmi_read;
    dmi->write = ch32_riscv_dmi_write;

    riscv_dmi_init(dmi);

    return true;
}

// EOF
