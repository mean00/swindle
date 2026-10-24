/**
 * @file bmp_rvTap_rp2040.cpp
 * @brief RISC-V DMI transport over esprit GPIO (RP2040)
 */

/*
  swindle: Gpio driver for Rvswd
  This code is derived from the blackmagic one but has been modified
  to aim at simplicity at the expense of performances (does not matter much though)
  (The compiler may mitigate that by inlining)

 */
/**
 * This is similar to the non rp2040
 *
 */
#include "esprit.h"

extern "C"
{
#include "jep106.h"
#include "riscv_debug.h"
}
#pragma GCC optimize("Ofast")
#include "bmp_pinmode.h"
#include "bmp_pinout.h"
#include "bmp_rvTap.h"
#include "esprit.h"
#include "lnRP2040_pio.h"
#include "ln_rp_pio.h"
#include "lnbmp_parity.h"
//--
extern void bmp_gpio_init();
extern rpPIO *swdpio;
extern rpPIO_SM *xsm;
extern uint32_t swd_frequency;

uint64_t rvswd_write_then_read(uint64_t tx_data, int tx_bits, int rx_bits)
{
    // Command word: [tx_bits:16][rx_bits:16]
    uint32_t cmd = ((uint32_t)tx_bits << 16) | (rx_bits & 0xFFFF);
    xsm->write(1, &cmd);

    // TX Data: shift to MSB alignment
    if (tx_bits > 0)
    {
        uint64_t aligned_tx = tx_data;
        if (tx_bits < 64)
        {
            aligned_tx <<= (64 - tx_bits);
        }
        uint32_t w0 = aligned_tx >> 32;
        xsm->write(1, &w0);
        if (tx_bits > 32)
        {
            uint32_t w1 = aligned_tx & 0xFFFFFFFF;
            xsm->write(1, &w1);
        }
    }

    // RX Data
    uint64_t rx_data = 0;
    if (rx_bits > 0)
    {
        uint32_t w0 = 0;
        xsm->waitRxReady();
        xsm->read(1, &w0);

        int expected_words = (rx_bits / 32) + 1;
        if (expected_words == 1)
        {
            rx_data = w0;
        }
        else if (expected_words == 2)
        {
            uint32_t w1 = 0;
            xsm->waitRxReady();
            xsm->read(1, &w1);
            if (rx_bits == 32)
            {
                rx_data = w0;
            }
            else
            {
                int rem_bits = rx_bits - 32;
                rx_data = ((uint64_t)w0 << rem_bits) | w1;
            }
        }
    }
    return rx_data;
}

/**
 * @brief Reset RISC-V DM via PIO (100 × 1-bit pulses).
 * @return true (always succeeds).
 */
bool rv_dm_reset()
{
    bmp_gpio_pinmode(BMP_PINMODE_RVSWD_RAW);

    uint32_t count = 100;
    xsm->write(1, &count);

    int words = (100 + 31) / 32;
    for (int i = 0; i < words; i++)
    {
        uint32_t ones = 0xFFFFFFFF;
        xsm->write(1, &ones);
    }
    xsm->waitTxEmpty();
    lnDelayMs(1); // Wait for PIO to finish shifting

    bmp_gpio_pinmode(BMP_PINMODE_RVSWD);
    return true;
}
/**
 * @brief Test function: write a known pattern over DMI.
 */
void rv_write_write(uint32_t value)
{
    Logger("Write Write\n");
    rvswd_write_then_read(0xaa55, 16, 0);
    lnDelayMs(1);
}
/* ---------------------------------------------------------------
 * Response window of a DMI write frame, sized in *time* rather than
 * in clocks: the DM needs ~8 us after the last request clock before
 * it drives the status (doc/rvswd_protocol.md). rvswd.pio spends
 * five PIO instructions per bit and the state machine runs at
 * 5 * swd_frequency (see setupPIO), so one bit clock is exactly
 * 1 / swd_frequency and 8 us is swd_frequency / 125000 clocks.
 * 16 clocks (the reference count of a fixed 1.85 MHz tap) is the
 * floor, 63 the ceiling: rvswd_write_then_read() decodes at most two
 * RX words, so the window must stay below 64 bits.
 * --------------------------------------------------------------- */
#define RVSWD_RESPONSE_MIN_CLOCKS 16
#define RVSWD_RESPONSE_MAX_CLOCKS 63

static int rp2040_rvswd_response_clocks()
{
    uint32_t clocks = swd_frequency / 125000; // 8 us at the current clock
    if (clocks < RVSWD_RESPONSE_MIN_CLOCKS)
        clocks = RVSWD_RESPONSE_MIN_CLOCKS;
    if (clocks > RVSWD_RESPONSE_MAX_CLOCKS)
        clocks = RVSWD_RESPONSE_MAX_CLOCKS;
    return (int)clocks;
}

/* Picked up by rvswd_template.h instead of its fixed-16 default. */
#define RVSWD_WRITE_RESPONSE_CLOCKS() rp2040_rvswd_response_clocks()

#include "rvswd_template.h"
// EOF
