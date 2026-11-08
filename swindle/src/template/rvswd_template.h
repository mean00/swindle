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
#include "platform_support.h"
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

/* Response window of a DMI write frame, in bit clocks. The protocol answer is four
 * status bits, but the DM only starts driving the wire a few microseconds after the
 * last request clock, so the window is sized by *time*, not by the four status bits:
 * four clocks were too short on a 144 MHz probe (and marginal on a 96 MHz one) and
 * came back as 0xf = "nothing driven". The status is the first four sampled bits,
 * the rest of the window idles high.
 *
 * The budget is 8 us of clocks (doc/rvswd_protocol.md: the response window is a time
 * budget, not a bit count). A fixed-rate tap just returns the reference count -- 16
 * clocks, 8.6 us at 1.85 MHz; a tap whose clock is programmable overrides this
 * before including the file so the budget stays 8 us when the clock is raised. */
#ifndef RVSWD_WRITE_RESPONSE_CLOCKS
#define RVSWD_WRITE_RESPONSE_CLOCKS() 16
#endif

#ifndef RVSWD_CONSUME_HW_TIMEOUT
#define RVSWD_CONSUME_HW_TIMEOUT() false
#endif

#define RVSWD_MAX_ADDRESS 0x7FU

// ---------------------------------------------------------------
// Diagnostics: the raw status word and the on-the-wire duration of the most
// recent failed frame. Reported by the retry layer when it gives up, so a
// failure says *why* (status 0xf = nothing came back, status busy, ...) and how
// fast the transport really runs, instead of only "after N attempts".
// ---------------------------------------------------------------
static volatile uint32_t rv_last_bad_status = 0;
static volatile uint32_t rv_last_bad_us = 0;

static inline bmp_wire_status_t rv_status_to_wire_status(const uint32_t status)
{
    if (status == 3U || status == 7U)
        return BMP_WIRE_OK;
    if (status == 0xFU)
        return BMP_WIRE_NO_RESPONSE;
    if (status == 0x1U || status == 0x5U)
        return BMP_WIRE_ACK_WAIT;
    if (status == 0x0U)
        return BMP_WIRE_PROTOCOL_ERR;
    return BMP_WIRE_ACK_FAULT;
}

bmp_wire_status_t LN_FAST_CODE rv_dm_write_status(uint32_t adr, uint32_t val);
bmp_wire_status_t LN_FAST_CODE rv_dm_read_status(uint32_t adr, uint32_t *output);
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
bmp_wire_status_t LN_FAST_CODE rv_dm_write_status(uint32_t adr, uint32_t val)
{
    if (adr > RVSWD_MAX_ADDRESS)
        return BMP_WIRE_PROTOCOL_ERR;
    ln_rv_tx_count++;
    SWD_TX_POLL();
    (void)RVSWD_CONSUME_HW_TIMEOUT();
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
    // RX: a whole response word is clocked (see RVSWD_WRITE_RESPONSE_CLOCKS()); the DMI
    // status is the first four sampled bits.
    const int rx_clocks = RVSWD_WRITE_RESPONSE_CLOCKS();
    uint32_t t0 = lnGetUs();
    uint64_t rx = rvswd_write_then_read(tx, 48, rx_clocks);
    if (RVSWD_CONSUME_HW_TIMEOUT())
    {
        rv_last_bad_status = BMP_WIRE_HW_TIMEOUT;
        rv_last_bad_us = lnGetUs() - t0;
        return BMP_WIRE_HW_TIMEOUT;
    }
    uint32_t status = (uint32_t)(rx >> (rx_clocks - 4)) & 0xF;

    if (status != 3 && status != 7)
    {
        // Quiet here: a non-OK status is usually "busy" (a system-bus access is still in
        // flight), which the DMI layer retries with a bounded budget (BMP 85). Log only
        // when the retries are exhausted; the reason is kept for that line.
        rv_last_bad_status = status;
        rv_last_bad_us = lnGetUs() - t0;
        return rv_status_to_wire_status(status);
    }
    return BMP_WIRE_OK;
}

bool LN_FAST_CODE rv_dm_write(uint32_t adr, uint32_t val)
{
    return rv_dm_write_status(adr, val) == BMP_WIRE_OK;
}
/**
 * @brief
 *
 * @param adr
 * @param output
 * @return bmp_wire_status_t
 */
bmp_wire_status_t LN_FAST_CODE rv_dm_read_status(uint32_t adr, uint32_t *output)
{
    if (!output || adr > RVSWD_MAX_ADDRESS)
        return BMP_WIRE_PROTOCOL_ERR;
    ln_rv_tx_count++;
    SWD_TX_POLL();
    (void)RVSWD_CONSUME_HW_TIMEOUT();
    uint64_t tx = (adr << 1) + 0; // 0 = read
    int parity1 = lnOddParity(tx);
    tx = (tx << 2) | (parity1 ? 3 : 0);

    // Padding: 4 bits of 1 (0001)
    tx = (tx << 4) | 1;

    // TX total = 14 bits
    // RX total = 32 (data) + 2 (parity) + 4 (status) = 38 bits
    uint32_t t0 = lnGetUs();
    uint64_t rx = rvswd_write_then_read(tx, 14, 38);
    if (RVSWD_CONSUME_HW_TIMEOUT())
    {
        *output = 0;
        rv_last_bad_status = BMP_WIRE_HW_TIMEOUT;
        rv_last_bad_us = lnGetUs() - t0;
        return BMP_WIRE_HW_TIMEOUT;
    }

    uint32_t status = rx & 0xF;
    *output = (rx >> 6) & 0xFFFFFFFF;

    if (status != 3 && status != 7)
    {
        // Quiet here: a non-OK status is usually "busy" (a system-bus access is still in
        // flight), which the DMI layer retries with a bounded budget (BMP 85). Log only
        // when the retries are exhausted; the reason is kept for that line.
        rv_last_bad_status = status;
        rv_last_bad_us = lnGetUs() - t0;
        return rv_status_to_wire_status(status);
    }
    return BMP_WIRE_OK;
}

bool LN_FAST_CODE rv_dm_read(uint32_t adr, uint32_t *output)
{
    return rv_dm_read_status(adr, output) == BMP_WIRE_OK;
}

/* RISC-V Debug Module (DM) registers */
#define DMCONTROL               0x10U
#define DMSTATUS                0x11U
#define DMDATA0                 0x04U
#define DMDATA1                 0x05U
#define DMABSTRACTCS            0x16U
#define DMCOMMAND               0x17U
#define DMCHIPID                0x7FU

/* DMCONTROL register bits */
#define DMCONTROL_DMACTIVE      (1U << 0)
#define DMCONTROL_HALTREQ       (1U << 31)

/* Abstract command & memory constants for probe sequence */
#define DM_ABSTRACT_CMD_MEM_READ32 0x02200000UL /* Access memory, 32-bit read */
#define CH32V_FLASH_CHIPID_ADDR    0x1FFFF704UL
#define RVSWD_CHIPID_INVALID_MASK  0x7FFFU

#define WR(a, b) rv_dm_write(a, b)
#define RD(a, ...) rv_dm_read(a, &out)

/**
 *
 *
 */
static bool rv_dm_probe(uint32_t *chip_id)
{
    *chip_id = 0;

    uint32_t out = 0;
    //
    // WCH DM initialization: activate DM first per Debug Spec 0.13.2 §3.4, then request halt
    //----------------------------------------------
    WR(DMCONTROL, DMCONTROL_DMACTIVE);
    WR(DMCONTROL, DMCONTROL_HALTREQ | DMCONTROL_DMACTIVE);
    uint32_t dm_status = 0;
    bool ok_status = rv_dm_read(DMSTATUS, &dm_status);

    if (RD(DMCHIPID)) // read DMCHIPID (0x7F)
    {
        *chip_id = out; // 0x203xxxx 0x303xxxx 0x305...
    }
    else
    {
        *chip_id = RVSWD_CHIPID_INVALID_MASK;
    }

    // If DMCHIPID did not return a valid ID, try reading FLASH CHIPID via abstract command
    if (((*chip_id) & RVSWD_CHIPID_INVALID_MASK) == RVSWD_CHIPID_INVALID_MASK)
    {
        WR(DMDATA1, CH32V_FLASH_CHIPID_ADDR);
        WR(DMCOMMAND, DM_ABSTRACT_CMD_MEM_READ32);
        // Wait for abstract command to complete
        bool abstract_ready = false;
        for (int i = 0; i < 20; i++)
        {
            if (RD(DMABSTRACTCS) && !(out & (1 << 12))) // busy bit 12 == 0
            {
                abstract_ready = true;
                break;
            }
        }
        if (abstract_ready && RD(DMDATA0))
        {
            uint32_t mem_id = out;
            (void)RD(DMDATA1);
            *chip_id = mem_id;
        }
    }

    Logger("RVSWD : rv_dm_probe ok_status=%d dmstatus=0x%x chip_id=0x%x\n",
           (int)ok_status, (unsigned)dm_status, (unsigned)*chip_id);
    return ((*chip_id) & RVSWD_CHIPID_INVALID_MASK) != RVSWD_CHIPID_INVALID_MASK;
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
 * back to the proven CTRLSTATUS busy-poll.
 * BMP 112: the retries are *spaced*. A DM that is still finishing a system-bus access
 * answers "busy", and back-to-back frames are much closer together than that, so
 * without the delay the attempts would all land inside the same busy window. */
#define RVSWD_DMI_MAX_ATTEMPTS 6U
#define RVSWD_DMI_RETRY_DELAY_US 100U

static bool ch32_riscv_dmi_read(riscv_dmi_s *const dmi, const uint32_t address, uint32_t *const value)
{
    bmp_wire_status_t status = BMP_WIRE_ACK_FAULT;
    for (uint32_t attempt = 0U; attempt < RVSWD_DMI_MAX_ATTEMPTS; ++attempt)
    {
        status = rv_dm_read_status(address, value);
        if (status == BMP_WIRE_OK)
        {
            dmi->fault = BMP_WIRE_OK;
            return true;
        }
        lnDelayUs(RVSWD_DMI_RETRY_DELAY_US);
    }
    Logger("DMI read failed Adr=0x%x after %u attempts (status=0x%x, %uus/frame, ws=%u)\n", address,
           (unsigned)RVSWD_DMI_MAX_ATTEMPTS, (unsigned)rv_last_bad_status, (unsigned)rv_last_bad_us,
           (unsigned)bmp_get_wait_state_c());
    dmi->fault = status;
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
    bmp_wire_status_t status = BMP_WIRE_ACK_FAULT;
    for (uint32_t attempt = 0U; attempt < RVSWD_DMI_MAX_ATTEMPTS; ++attempt)
    {
        status = rv_dm_write_status(address, value);
        if (status == BMP_WIRE_OK)
        {
            dmi->fault = BMP_WIRE_OK;
            return true;
        }
        lnDelayUs(RVSWD_DMI_RETRY_DELAY_US);
    }
    Logger("DMI write failed Adr=0x%x after %u attempts (status=0x%x, %uus/frame, ws=%u)\n", address,
           (unsigned)RVSWD_DMI_MAX_ATTEMPTS, (unsigned)rv_last_bad_status, (unsigned)rv_last_bad_us,
           (unsigned)bmp_get_wait_state_c());
    dmi->fault = status;
    return false;
}
extern "C"
{
    extern "C" bmp_wire_status_t bmp_rv_dm_read_status_c(uint8_t adr, uint32_t *value)
    {
        return rv_dm_read_status(adr, value);
    }
    extern "C" bmp_wire_status_t bmp_rv_dm_write_status_c(uint8_t adr, uint32_t value)
    {
        return rv_dm_write_status(adr, value);
    }
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
        return rv_dm_read_status(adr, value) == BMP_WIRE_OK;
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
        return rv_dm_write_status(adr, value) == BMP_WIRE_OK;
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
/* Session arena (malloc_used.md): the riscv_dmi_s is the transport object of
 * the session, so it comes from the arena's transport region instead of the
 * heap.  Declared here because these templates are C++ TUs, which never see
 * the calloc/malloc/free shim (session_arena_shim.h). */
extern "C" void *swindle_session_transport_alloc(size_t size);

/* Resynchronise the wire after a target reset.
 *
 * The ndmreset a `mon reset` performs reaches the core through the same serial frame the
 * probe is still clocking, so the DM stops answering *inside* that exchange and the
 * transport stays one frame behind from then on: every later read comes back shifted -
 * zeros, or the previous content - with a status nibble that still decodes as OK, which is
 * why a session after a reset could report a sane-looking `dmstatus` for a completely dead
 * core. Re-running the start pulse train (`rv_dm_reset()`) is what resynchronises it, and
 * `invalidate_caches` is the hook the DMI layer calls right after a reset, so wire the
 * resync there instead of leaving the session desynchronised. */
extern "C" void rvswd_dmi_invalidate_c(riscv_dmi_s *dmi)
{
    (void)dmi;
    rv_dm_reset();
}

#define RVSWD_POST_SWITCH_SETTLE_MS 5U

extern "C" bool sdi_probe_and_enable_2wire();
extern "C" bool sdi_scan();

extern "C" bool rvswdp_scan()
{
    uint32_t id = 0;
    rv_dm_start();
    target_list_free();
    if (!rv_dm_probe(&id))
    {
        Logger("RVSWD : 2-wire probe failed, attempting SDI unlock & switch to 2-wire\n");
        if (sdi_probe_and_enable_2wire())
        {
            // Switch probe pins and globals back to RVSWD mode
            bmp_gpio_pinmode(BMP_PINMODE_RVSWD);
            rv_dm_start();
            lnDelayMs(RVSWD_POST_SWITCH_SETTLE_MS);

            if (rv_dm_probe(&id))
            {
                Logger("RVSWD : target successfully switched to 2-wire, found 0x%x\n", (unsigned)id);
            }
            else
            {
                Logger("RVSWD : 2-wire probe still failing after switch, falling back to SDI\n");
                return sdi_scan();
            }
        }
        else
        {
            bmp_gpio_pinmode(BMP_PINMODE_RVSWD);
            return false;
        }
    }
    Logger("WCH : found 0x%x device\n", id);
    /* Transport object of this session: from the session arena's transport
     * region, not the heap.  It is reclaimed when the target list is freed,
     * which also fixes the old `new riscv_dmi_s` leak (nothing ever deleted it,
     * 40 B per scan). */
    riscv_dmi_s *dmi = (riscv_dmi_s *)swindle_session_transport_alloc(sizeof(*dmi));
    if (!dmi)
    { /* allocation failed: heap and arena exhausted */
        Logger("calloc: failed in %s\n", __func__);
        return false;
    }
    memset(dmi, 0, sizeof(*dmi));
    dmi->designer_code = JEP106_MANUFACTURER_WCH;
    dmi->version = RISCV_DEBUG_0_13; /* Assumption, unverified */
    /* WCH RVSWD/SDI carry a 7-bit DMI address field (start+7addr+RW header),
     * like wchlink_riscv_dtm.c; unused by the taps (fixed-frame drivers), but
     * describe the bus correctly for any width-aware consumer. */
    dmi->address_width = 7U;
    dmi->read = ch32_riscv_dmi_read;
    dmi->write = ch32_riscv_dmi_write;
    dmi->invalidate_caches = rvswd_dmi_invalidate_c;

    riscv_dmi_init(dmi);

    return true;
}

extern "C" bool rvswd_scan()
{
    return rvswdp_scan();
}

// EOF
