/**
 * @file bmp_reset_pin.cpp
 * @brief Platform reset-pin glue and frequency accessor.
 *
 * Connects the SwdReset object (pReset) to the BMP platform hooks
 * for nRST assertion/de-assertion and SWD clock frequency queries.
 */
#include "esprit.h"
#include "lnBMP_pins.h"
#include "lnBMP_reset.h"
// clang-format on
extern SwdReset *pReset;
extern uint32_t swd_frequency;

/** @brief Test routine — toggles reset 5 times with 2 s delay. */
extern "C" void resetTest(void)
{
    for (int i = 0; i < 5; i++)
    {
        pReset->off();
        lnDelayMs(2000);
        pReset->on();
        lnDelayMs(2000);
    }
}

/**
 * @brief Get current nRST state from the platform reset driver.
 * @return true if asserted, false otherwise.
 */
extern "C" bool platform_nrst_get_val(void)
{
    return pReset->state();
}

/**
 * @brief Assert or de-assert the target nRST line.
 * @param assrt true → hold reset, false → release.
 */
extern "C" void platform_nrst_set_val_internal(bool assrt)
{
    if (assrt)
    {
        Logger("Reset on\n");
        pReset->on();
    }
    else
    {
        Logger("Reset off\n");
        pReset->off();
    }
}

/**
 * @brief Get the current SWD frequency.
 * @return SWD clock frequency in Hz.
 */
extern "C" uint32_t bmp_get_frequency_c()
{
    return swd_frequency;
}
// EOF

//
