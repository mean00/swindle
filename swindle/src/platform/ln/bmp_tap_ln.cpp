/**
 * @file bmp_tap_ln.cpp
 * @brief LN platform tap: ownership of the shared debug pins (PB8/PC3/NRST), the
 *        debug pin-mode switch, and SWD frequency / wait-state control.
 */

#include "bmp_tap_ln.h"
#include "bmp_pinmode.h"
#include "bmp_pinout.h"
#include "bmp_swdio_ln.h"
/* SDI pin hand-over (sdi_pinmode_enter/leave): the seam header declares it, the
 * SDI transport defines it, and bmp_riscv_extra_stubs.cpp makes it a no-op in a
 * build without SDI. Nothing else in this file is SDI. */
#include "bmp_riscv_extra.h"
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

/* SDI pin hand-over (bmp_sdiTap_ln_bitbang_riscv_extra.cpp /
 * bmp_sdiTap_ln_riscv_extra.cpp). SDI owns PB8 for the whole of a session
 * (released open drain) and is the only protocol whose pins do not double as the
 * SWD ones, so it has to be told when its session begins and when it ends. Both
 * hooks come from bmp_riscv_extra.h. */

/**
 * @brief Select the debug pins for @p pioMode.
 *
 * This is the only place that knows about every pin mode: SWD and RVSWD
 * reconfigure PB8/PC3 on their own (swdptap_init(), bmp_rvTap_ln.cpp's scan
 * sets rSWDIO output/input as it goes), so SDI is the single mode that needs an
 * explicit hand-over. Any other mode - including BMP_PINMODE_NONE - ends a
 * running SDI session.
 */
void bmp_gpio_pinmode(bmp_pin_mode pioMode)
{
    if (pioMode == BMP_PINMODE_SDI)
        sdi_pinmode_enter();
    else
        sdi_pinmode_leave();
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
