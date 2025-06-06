#include "esprit.h"
#include "lnBMP_pinout.h"
#include "lnBMP_swdio.h"
#include "lnCpuID.h"
#include "math.h"
extern uint32_t swd_delay_cnt;
extern uint32_t swd_frequency;
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
    case lnCpuID::LN_MCU_GD32: // assume this is a GD32F303 at 96 Mhz
        alpha = 7.681 * 1000000.;
        beta = 4.5;
        break;
    case lnCpuID::LN_MCU_CH32: // assume this is a CH32V3 at 140 Mhz
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
//
