#include "esprit.h"
#include "lnBMP_pins.h"
#include "lnBMP_reset.h"
// clang-format on
extern SwdReset *pReset;
extern uint32_t swd_frequency;
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
 */
extern "C" bool platform_nrst_get_val(void)
{
    return pReset->state();
}
/*
 */
extern "C" void platform_nrst_set_val_internal(bool assrt)
{
    if (assrt) // force reset to low
    {
        Logger("Reset on\n");
        pReset->on();
    }
    else // release reset
    {
        Logger("Reset off\n");
        pReset->off();
    }
}
/*
 *
 */
extern "C" uint32_t bmp_get_frequency_c()
{
    return swd_frequency;
}


//
