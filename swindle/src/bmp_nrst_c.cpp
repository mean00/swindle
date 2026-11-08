/**
 * @file bmp_nrst_c.cpp
 * @brief C++ implementation of the target nRST entry points (SwdReset / pReset).
 *
 * Only compiled for the platforms that still drive the reset line from C++ —
 * RP2040/RP2350 and ESP32 (swindle/swindle_target.cmake). On the LN probes (GD32 /
 * CH32) the pin comes from the probe board table and is driven in Rust
 * (swindle/rs/rs_swindle/src/reset_pin.rs), which defines the very same symbols:
 * compiling this file there would be a duplicate definition.
 */
#include "esprit.h"
#include "lnBMP_reset.h"

extern SwdReset *pReset;

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
// EOF
