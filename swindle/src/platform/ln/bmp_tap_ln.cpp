/**
 * @file bmp_tap_ln.cpp
 * @brief SWD TAP initialisation, frequency and wait-state control (LN targets)
 */

#include "bmp_tap_ln.h"
#include "bmp_pinout.h"
#include "bmp_swdio_ln.h"
#include "esprit.h"
#include "lnCpuID.h"
#include "math.h"

extern void gmp_gpio_init_adc();

uint32_t swd_delay_cnt = 4;
uint32_t swd_frequency = 1000 * 1000; // 1MHz default
SwdDirectionPin *rSWDIO;
SwdWaitPin *rSWCLK;
SwdReset *pReset;

/**
 * @brief Set the SWD wait-state (delay-loop count).
 * @param ws Number of NOPs per bit-clock.
 */
extern "C" void bmp_set_wait_state_c(uint32_t ws)
{
    swd_delay_cnt = ws;
    return;
}
/**
 * @brief Set the SWD clock frequency.
 *
 * Converts the requested frequency (Hz) into a wait-state count
 * based on the MCU vendor's CPU speed.
 * @param fq Desired SWCLK frequency in Hz.
 */
extern "C" void bmp_set_frequency_c(uint32_t fq)
{
    float alpha = 0, beta = 0;
    if (fq < 10)
    {
        Logger("Invalid frequency\n");
        return;
    }
    switch (lnCpuID::vendor())
    {
    case lnCpuID::LN_VENDOR_GD: // assume this is a GD32F303 at 96 Mhz
        alpha = 7.681 * 1000000.;
        beta = 4.5;
        break;
    case lnCpuID::LN_VENDOR_WCH: // assume this is a CH32V3 at 140 Mhz
        alpha = 16.668 * 1000000.;
        beta = 5.0;
        break;
    default:
        xAssert(0);
        break;
    }
    // convert fq to wait state
    float wf = ((alpha) / (float)fq) - beta;
    if (wf < 0.0f)
        wf = 0.f;

    // get wf which is the wait state as rounded int
    wf = floor(wf + 0.49f);
    // now reinvert it to update the actual fq
    float gf = (alpha) / (wf + beta);
    swd_frequency = gf;
    bmp_set_wait_state_c((uint32_t)wf);
}
/**
 * @brief Get the current SWD wait-state count.
 * @return Number of NOPs per bit-clock.
 */
extern "C" uint32_t bmp_get_wait_state_c()
{
    return swd_delay_cnt;
}
/**
 * @brief Initialise SWDIO/SWCLK/reset GPIOs once at startup.
 */
void bmp_gpio_init_once()
{
    rSWDIO = new SwdDirectionPin(TSWDIO_PIN, TDIRECTION_PIN);
    rSWCLK = new SwdWaitPin(TSWDCK_PIN); // automatically add delay after toggle
    pReset = new SwdReset(TRESET_PIN);   // automatically add delay after toggle
    pReset->setup();
    rSWDIO->hiZ();
    rSWDIO->hiZ();
    rSWCLK->hiZ();
    rSWCLK->hiZ();
    pReset->hiZ(); // hi-z by default
    pReset->off(); // hi-z by default
    gmp_gpio_init_adc();
}
/**
 * @brief Begin an SWD debug session: drive SWDIO/SWCLK, release reset.
 */
extern "C" void bmp_io_begin_session()
{
    rSWDIO->on();
    rSWDIO->output();
    rSWCLK->clockOn();
    rSWCLK->output();
    pReset->off(); // hi-z by default
}
/**
 * @brief End an SWD debug session: return pins to Hi-Z.
 */
extern "C" void bmp_io_end_session()
{
    rSWDIO->hiZ();
    rSWDIO->hiZ();
    rSWCLK->hiZ();
    rSWCLK->hiZ();
    pReset->off(); // hi-z by default
}
/**
 * @brief Disable frequency scaling (no-op for LN platform).
 */
void disableFq()
{
}
/**
 * @brief Extra wait-state adjustment (no-op for LN platform).
 */
void bmp_extraSetWaitState()
{
}

//
