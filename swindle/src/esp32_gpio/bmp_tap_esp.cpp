/*
 *
 */
#include "esprit.h"
#include "bmp_pinout.h"
#include "bmp_swdio_esp.h"
#include "lnCpuID.h"
#include "math.h"
#include "bmp_tap_esp.h"

extern void gmp_gpio_init_adc();

uint32_t swd_delay_cnt = 10;
uint32_t swd_frequency = 1000 * 1000; // 1Mhz by default
SwdDirectionPin *rSWDIO;
SwdWaitPin *rSWCLK;
SwdReset *pReset;
/**
 *
 */
extern "C" void bmp_set_wait_state_c(uint32_t ws)
{
    swd_delay_cnt = ws;
    return;
}
/**
 * @brief
 *
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
 * @brief
 *
 */
extern "C" uint32_t bmp_get_wait_state_c()
{
    return swd_delay_cnt;
}
/**

*/
void bmp_gpio_init_once()
{
    rSWDIO = new SwdDirectionPin(TSWDIO_PIN);
    rSWCLK = new SwdWaitPin(TSWDCK_PIN); // automatically add delay after toggle
    pReset = new SwdReset(TRESET_PIN);   // automatically add delay after toggle
    pReset->setup();
    rSWDIO->hiZ();
    pReset->off(); // hi-z by default

    gmp_gpio_init_adc();
}
/**
 * @brief
 *
 */
void bmp_io_begin_session()
{
    rSWDIO->on();
    rSWDIO->output();
    rSWCLK->clockOn();

    pReset->off(); // hi-z by default
}
/**
 * @brief
 *
 */
void bmp_io_end_session()
{
    rSWDIO->hiZ();
    rSWDIO->hiZ();
    pReset->off(); // hi-z by default
}
//
