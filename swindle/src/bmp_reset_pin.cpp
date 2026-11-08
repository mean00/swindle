/**
 * @file bmp_reset_pin.cpp
 * @brief SWD clock frequency accessor.
 *
 * The target nRST line used to be driven from here too. On the LN probes (GD32 /
 * CH32) it now comes from the probe board table and is driven in Rust
 * (swindle/rs/rs_swindle/src/reset_pin.rs, esprit GPIO trait), while the C++
 * implementation the RP2040/RP2350 and ESP32 builds still need (through SwdReset
 * / pReset) is in bmp_nrst_c.cpp, which only those platforms compile. That keeps
 * exactly one definition of platform_nrst_set_val_internal() per firmware.
 */
#include "esprit.h"

extern uint32_t swd_frequency;

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
