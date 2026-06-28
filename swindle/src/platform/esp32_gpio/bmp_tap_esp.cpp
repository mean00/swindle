/*
 *
 */
/**
 * @file bmp_tap_esp.cpp
 * @brief ESP32 TAP initialisation (standard GPIO)
 */

#include "bmp_tap_esp.h"
#include "bmp_pinout.h"
#include "bmp_swdio_esp.h"
#include "esprit.h"
#include "lnCpuID.h"
#include "math.h"

extern void gmp_gpio_init_adc();

uint32_t swd_delay_cnt = 2;
uint32_t swd_frequency = 1000 * 1000; // 1Mhz by default
SwdDirectionPin *rSWDIO;
SwdWaitPin *rSWCLK;
SwdReset *pReset;
/**
 * @brief Set the SWD wait-state (number of NOPs per bit-clock).
 * @param ws Wait-state count.
 */
extern "C" void bmp_set_wait_state_c(uint32_t ws)
{
    swd_delay_cnt = ws;
    return;
}
/**
 * @brief Set the SWD clock frequency.
 *
 * Converts Hz to wait-state count using ESP32-specific calibration.
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
    alpha = 2.3 * 1000000.;
    beta = 0.0;
    // convert fq to wait state
    float wf = ((alpha) / (float)fq) - beta;
    if (wf < 0.0)
        wf = 0.;

    // get wf which is the wait state as rounded int
    wf = floor(wf + 0.49);
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
 * @brief Initialise SWD GPIOs once at startup.
 */
void bmp_gpio_init_once()
{
    rSWDIO = new SwdDirectionPin(TSWDIO_PIN);
    rSWCLK = new SwdWaitPin(TSWDCK_PIN); // automatically add delay after toggle
    pReset = new SwdReset(TRESET_PIN);   // automatically add delay after toggle
    pReset->setup();
    rSWDIO->output();
    pReset->off(); // hi-z by default
    rSWCLK->off();

    gmp_gpio_init_adc();
}
/**
 * @brief Begin an SWD session: drive SWDIO/SWCLK.
 */
void bmp_io_begin_session()
{
    rSWDIO->on();
    rSWDIO->output();
    rSWCLK->clockOn();

    pReset->off(); // hi-z by default
}
/**
 * @brief End an SWD session: return SWDIO to Hi-Z.
 */
void bmp_io_end_session()
{
    rSWDIO->hiZ();
    rSWDIO->hiZ();
    pReset->off(); // hi-z by default
}
//
